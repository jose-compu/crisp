//! Lower analyzed AST to CIR (spec §17.1).

use crate::node::*;
use crate::source_map::SourceMap;
use crate::synthesize::{lower_enum, lower_struct, synthesize_shape_trait};
use crate::ty::CirTy;
use crisp_ast::expr::{BinaryOp, Block, Expr, ExprKind, Stmt, StringPart};
use crisp_ast::item::{FunctionDef, Item, SourceFile, TypeBody};
use crisp_ast::pat::PatKind;
use crisp_errors::{ErrorPass, ErrorResult};
use crisp_ownership::{FallbackKind, OwnershipMode, OwnershipPass, OwnershipResult};
use crisp_regions::{RegionPass, RegionResult};
use crisp_resolve::module::{ModuleGraph, load_module_graph};
use crisp_typeck::{InferredSig, Ty, TypeChecker, TypedCrate};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CirError {
    #[error("[E0070] ownership error: {0}")]
    Ownership(#[from] crisp_ownership::OwnershipError),
    #[error("[E0071] type error: {0}")]
    Type(#[from] crisp_typeck::TypeError),
    #[error("[E0072] region error: {0}")]
    Region(#[from] crisp_regions::RegionError),
    #[error("[E0073] error pass: {0}")]
    Errors(#[from] crisp_errors::ErrorPassError),
    #[error("[E0074] resolve error: {0}")]
    Resolve(#[from] crisp_resolve::ResolveError),
}

pub struct CirBuilder;

impl CirBuilder {
    pub fn build_crate(crate_root: &Path) -> Result<CirCrate, CirError> {
        let graph = load_module_graph(crate_root)?;
        let typed = TypeChecker::check_crate(crate_root)?;
        let ownership = OwnershipPass::analyze_crate(crate_root)?;
        let regions = RegionPass::assign_crate(crate_root)?;
        let errors = ErrorPass::analyze_crate(crate_root)?;
        let package_name = read_package_name(crate_root).unwrap_or_else(|| "crisp_app".into());
        Self::build_from_analysis(&graph, &typed, &ownership, &regions, &errors, &package_name)
    }

    pub fn build_from_analysis(
        graph: &ModuleGraph,
        typed: &TypedCrate,
        ownership: &OwnershipResult,
        regions: &RegionResult,
        errors: &ErrorResult,
        package_name: &str,
    ) -> Result<CirCrate, CirError> {
        let struct_types = collect_struct_types(graph, typed);
        let mut all_structs: Vec<(String, CirStruct)> = Vec::new();
        for node in graph.modules.values() {
            for ast_item in &node.ast.items {
                if let Item::TypeDef(td) = ast_item {
                    if let TypeBody::Struct(_) = &td.body {
                        let field_types = struct_types.get(&td.name.name);
                        let st = lower_struct(td, field_types.unwrap_or(&BTreeMap::new()));
                        all_structs.push((node.module_path.clone(), st));
                    }
                }
            }
        }
        let fn_modules = collect_fn_modules(graph);
        let struct_fields = collect_struct_field_names(&all_structs);
        let mut shape_traits = Vec::new();
        let mut modules = Vec::new();

        for node in graph.modules.values() {
            let mut items = Vec::new();
            for ast_item in &node.ast.items {
                match ast_item {
                    Item::TypeDef(td) => match &td.body {
                        TypeBody::Struct(_) => {
                            let st = all_structs
                                .iter()
                                .find(|(_, s)| s.name == td.name.name)
                                .map(|(_, s)| s.clone())
                                .expect("struct pre-pass");
                            items.push(CirItem::Struct(st));
                        }
                        TypeBody::Enum(_) => {
                            if let Some(en) = lower_enum(td) {
                                items.push(CirItem::Enum(en));
                            }
                        }
                        TypeBody::Alias(ty) => {
                            items.push(CirItem::Alias {
                                name: td.name.name.clone(),
                                is_pub: td.is_pub,
                                ty: ast_type_to_cir_ty(ty),
                                span: td.span,
                            });
                        }
                    },
                    Item::ShapeDef(shape) => {
                        shape_traits.push(synthesize_shape_trait(shape, &all_structs));
                    }
                    Item::Function(f) => {
                        let key = format!("{}::{}", node.module_path, f.name.name);
                        if let (Some(o), Some(t), Some(e)) = (
                            ownership.signatures.get(&key),
                            typed.signatures.get(&key),
                            errors.signatures.get(&key),
                        ) {
                            let lt = regions.lifetimes.get(&key);
                            items.push(CirItem::Function(lower_function(
                                f,
                                o,
                                t,
                                e,
                                lt,
                                &node.module_path,
                                typed,
                                ownership,
                                errors,
                                &struct_fields,
                                &fn_modules,
                            )));
                        }
                    }
                    Item::Impl(ib) => {
                        let ty_name = type_name_from_ast(&ib.ty);
                        let mut fns = Vec::new();
                        for f in &ib.items {
                            let key = format!("{}::{}", node.module_path, f.name.name);
                            if let (Some(o), Some(t), Some(e)) = (
                                ownership.signatures.get(&key),
                                typed.signatures.get(&key),
                                errors.signatures.get(&key),
                            ) {
                                let lt = regions.lifetimes.get(&key);
                                fns.push(lower_function(
                                    f,
                                    o,
                                    t,
                                    e,
                                    lt,
                                    &node.module_path,
                                    typed,
                                    ownership,
                                    errors,
                                    &struct_fields,
                                    &fn_modules,
                                ));
                            }
                        }
                        items.push(CirItem::Impl(CirImpl {
                            trait_name: ib.trait_name.as_ref().map(|i| i.name.clone()),
                            ty_name,
                            functions: fns,
                            span: ib.span,
                        }));
                    }
                    _ => {}
                }
            }
            modules.push(CirModule {
                path: node.module_path.clone(),
                items,
            });
        }

        Ok(CirCrate {
            package_name: package_name.to_string(),
            modules,
            crisp_error: errors.crisp_error.clone(),
            shape_traits,
            source_map: SourceMap::default(),
        })
    }
}

fn read_package_name(crate_root: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(crate_root.join("crisp.toml")).ok()?;
    let table: toml::Table = manifest.parse().ok()?;
    table
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_string)
}

fn collect_struct_types(
    graph: &ModuleGraph,
    typed: &TypedCrate,
) -> BTreeMap<String, BTreeMap<String, Ty>> {
    let _ = (graph, typed);
    BTreeMap::new()
}

fn type_name_from_ast(ty: &crisp_ast::ty::Type) -> String {
    use crisp_ast::ty::TypeKind;
    match &ty.kind {
        TypeKind::Named(id) => id.name.clone(),
        _ => "Unknown".into(),
    }
}

fn ast_type_to_cir_ty(ty: &crisp_ast::ty::Type) -> CirTy {
    use crisp_ast::ty::TypeKind;
    match &ty.kind {
        TypeKind::Named(id) => match id.name.as_str() {
            "int" => CirTy::Int,
            "uint" => CirTy::UInt,
            "float" => CirTy::Float,
            "bool" => CirTy::Bool,
            "str" => CirTy::Str,
            other => CirTy::Named {
                name: other.to_string(),
                args: vec![],
            },
        },
        _ => CirTy::Error,
    }
}

fn collect_fn_modules(graph: &ModuleGraph) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for node in graph.modules.values() {
        for item in &node.ast.items {
            if let Item::Function(f) = item {
                map.insert(f.name.name.clone(), node.module_path.clone());
            }
        }
    }
    map
}

fn collect_struct_field_names(
    structs: &[(String, CirStruct)],
) -> BTreeMap<String, Vec<String>> {
    structs
        .iter()
        .map(|(_, s)| {
            (
                s.name.clone(),
                s.fields.iter().map(|f| f.name.clone()).collect(),
            )
        })
        .collect()
}

fn lower_function(
    def: &FunctionDef,
    osig: &crisp_ownership::OwnershipSignature,
    tsig: &InferredSig,
    esig: &crisp_errors::ErrorSig,
    lt: Option<&crisp_regions::LifetimeSig>,
    module: &str,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    errors: &ErrorResult,
    struct_fields: &BTreeMap<String, Vec<String>>,
    fn_modules: &BTreeMap<String, String>,
) -> CirFunction {
    let params: Vec<CirParam> = osig
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, mode))| {
            let ty = tsig
                .params
                .get(i)
                .map(|(_, t)| CirTy::from_ty(t))
                .unwrap_or(CirTy::Error);
            let lifetime = lt.and_then(|l| l.param_lifetimes.get(i).cloned().flatten());
            CirParam {
                name: name.clone(),
                ty,
                mode: *mode,
                lifetime,
                span: def.span,
            }
        })
        .collect();

    let ret = if esig.fallible {
        CirTy::Result {
            ok: Box::new(CirTy::from_ty(&tsig.ret)),
            err: "CrispError".into(),
        }
    } else {
        CirTy::from_ty(&tsig.ret)
    };

    let mut locals: BTreeMap<String, CirTy> = BTreeMap::new();
    for (i, (name, _)) in tsig.params.iter().enumerate() {
        if let Some((_, t)) = tsig.params.get(i) {
            locals.insert(name.clone(), CirTy::from_ty(t));
        }
    }

    let body = lower_body(
        &def.body,
        module,
        osig,
        typed,
        ownership,
        errors,
        &mut locals,
        esig.fallible,
        struct_fields,
        fn_modules,
    );

    CirFunction {
        name: def.name.name.clone(),
        is_pub: def.is_pub,
        is_main: def.name.name == "main",
        params,
        ret,
        fallible: esig.fallible,
        lifetimes: lt.cloned(),
        body,
        span: def.span,
    }
}

fn lower_body(
    expr: &Expr,
    module: &str,
    osig: &crisp_ownership::OwnershipSignature,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    errors: &ErrorResult,
    locals: &mut BTreeMap<String, CirTy>,
    fn_fallible: bool,
    struct_fields: &BTreeMap<String, Vec<String>>,
    fn_modules: &BTreeMap<String, String>,
) -> CirBlock {
    match &expr.kind {
        ExprKind::Block(b) => lower_block(
            b,
            module,
            osig,
            typed,
            ownership,
            errors,
            locals,
            fn_fallible,
            struct_fields,
            fn_modules,
        ),
        other => {
            let tail = lower_expr(
                &Expr {
                    kind: other.clone(),
                    span: expr.span,
                },
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
            );
            CirBlock {
                stmts: vec![],
                tail: Some(Box::new(tail)),
                span: expr.span,
            }
        }
    }
}

fn lower_block(
    block: &Block,
    module: &str,
    osig: &crisp_ownership::OwnershipSignature,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    errors: &ErrorResult,
    locals: &mut BTreeMap<String, CirTy>,
    fn_fallible: bool,
    struct_fields: &BTreeMap<String, Vec<String>>,
    fn_modules: &BTreeMap<String, String>,
) -> CirBlock {
    let mut stmts = Vec::new();
    for stmt in &block.stmts {
        match stmt {
            Stmt::Bind { pat, value, .. } => {
                if let PatKind::Ident(name) = &pat.kind {
                    let mut lowered = lower_expr(
                        value, module, osig, typed, ownership, errors, locals,
                        struct_fields, fn_modules,
                    );
                    if should_clone_at_bind(osig, &name.name, value) {
                        lowered = CirExpr::Clone {
                            expr: Box::new(lowered),
                            span: value.span,
                        };
                    }
                    let ty = infer_expr_ty(value, locals, typed, module);
                    locals.insert(name.name.clone(), ty);
                    stmts.push(CirStmt::Let {
                        name: name.name.clone(),
                        value: lowered,
                        span: value.span,
                    });
                }
            }
            Stmt::Expr(e) => {
                stmts.push(CirStmt::Expr(lower_expr(
                    e, module, osig, typed, ownership, errors, locals,
                    struct_fields, fn_modules,
                )));
            }
            Stmt::Assign { target, value, .. } => {
                stmts.push(CirStmt::Assign {
                    target: target.name.clone(),
                    value: lower_expr(
                        value, module, osig, typed, ownership, errors, locals,
                        struct_fields, fn_modules,
                    ),
                    span: value.span,
                });
            }
        }
    }
    let tail = block.tail.as_ref().map(|e| {
        lower_expr(
            e, module, osig, typed, ownership, errors, locals,
            struct_fields, fn_modules,
        )
    });
    let _ = fn_fallible;
    CirBlock {
        stmts,
        tail: tail.map(Box::new),
        span: block.span,
    }
}

fn lower_expr(
    expr: &Expr,
    module: &str,
    osig: &crisp_ownership::OwnershipSignature,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    errors: &ErrorResult,
    locals: &BTreeMap<String, CirTy>,
    struct_fields: &BTreeMap<String, Vec<String>>,
    fn_modules: &BTreeMap<String, String>,
) -> CirExpr {
    use crate::node::{CirFormatPart, CirExpr as E};
    match &expr.kind {
        ExprKind::Ident(id) => E::Ident {
            name: id.name.clone(),
            ty: locals.get(&id.name).cloned().unwrap_or(CirTy::Error),
            span: expr.span,
        },
        ExprKind::Int(n) => E::Int {
            value: *n,
            span: expr.span,
        },
        ExprKind::Str(parts) => {
            let has_expr = parts.0.iter().any(|p| matches!(p, StringPart::Expr(_)));
            if has_expr {
                E::Format {
                    parts: parts
                        .0
                        .iter()
                        .map(|p| match p {
                            StringPart::Lit(l) => CirFormatPart::Lit(l.clone()),
                            StringPart::Expr(e) => CirFormatPart::Expr(lower_expr(
                                e, module, osig, typed, ownership, errors, locals,
                                struct_fields, fn_modules,
                            )),
                        })
                        .collect(),
                    span: expr.span,
                }
            } else {
                let mut s = String::new();
                for p in &parts.0 {
                    if let StringPart::Lit(l) = p {
                        s.push_str(l);
                    }
                }
                E::Str {
                    value: s,
                    span: expr.span,
                }
            }
        }
        ExprKind::Field { base, field } => E::Field {
            base: Box::new(lower_expr(
                base,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
            )),
            field: field.name.clone(),
            ty: CirTy::Error,
            span: expr.span,
        },
        ExprKind::Binary { op, left, right } => {
            let l = lower_expr(left, module, osig, typed, ownership, errors, locals, struct_fields, fn_modules);
            let r = lower_expr(right, module, osig, typed, ownership, errors, locals, struct_fields, fn_modules);
            let cir_op = match op {
                BinaryOp::Concat => CirBinOp::Concat,
                BinaryOp::Add => CirBinOp::Add,
                BinaryOp::Sub => CirBinOp::Sub,
                BinaryOp::Mul => CirBinOp::Mul,
                BinaryOp::Div => CirBinOp::Div,
                BinaryOp::Eq => CirBinOp::Eq,
                BinaryOp::Lt => CirBinOp::Lt,
                BinaryOp::Gt => CirBinOp::Gt,
                _ => CirBinOp::Add,
            };
            E::BinOp {
                op: cir_op,
                left: Box::new(l),
                right: Box::new(r),
                ty: CirTy::Str,
                span: expr.span,
            }
        }
        ExprKind::Call { func, args } => {
            if let ExprKind::Ident(id) = &func.kind {
                if (id.name == "print" || id.name == "log") && args.len() == 1 {
                    return E::Print {
                        arg: Box::new(lower_expr(
                            &args[0], module, osig, typed, ownership, errors, locals,
                            struct_fields, fn_modules,
                        )),
                        span: expr.span,
                    };
                }
                let callee_module = fn_modules
                    .get(&id.name)
                    .cloned()
                    .unwrap_or_else(|| module.to_string());
                let key = format!("{callee_module}::{}", id.name);
                let fallible = errors
                    .signatures
                    .get(&key)
                    .map(|s| s.fallible)
                    .unwrap_or(false);
                let ret_ty = typed
                    .signatures
                    .get(&key)
                    .map(|s| CirTy::from_ty(&s.ret))
                    .unwrap_or(CirTy::Unit);
                let callee_osig = ownership.signatures.get(&key);
                let call_args: Vec<CirCallArg> = args
                    .iter()
                    .enumerate()
                    .map(|(i, arg)| {
                        let mut lowered = lower_expr(
                            arg, module, osig, typed, ownership, errors, locals,
                            struct_fields, fn_modules,
                        );
                        let mode = callee_osig
                            .and_then(|c| c.params.get(i).map(|(_, m)| *m))
                            .unwrap_or(OwnershipMode::Borrow);
                        if matches!(mode, OwnershipMode::Borrow)
                            && matches!(lowered, E::Ident { .. })
                        {
                            lowered = E::Borrow {
                                expr: Box::new(lowered),
                                mutable: false,
                                span: arg.span,
                            };
                        }
                        CirCallArg {
                            expr: lowered,
                            mode,
                        }
                    })
                    .collect();
                return E::Call {
                    callee: id.name.clone(),
                    module: callee_module,
                    args: call_args,
                    ty: ret_ty,
                    fallible,
                    span: expr.span,
                };
            }
            E::Unit { span: expr.span }
        }
        ExprKind::StructLit { name, fields } => {
            let all = struct_fields
                .get(&name.name)
                .cloned()
                .unwrap_or_else(|| fields.iter().map(|f| f.name.name.clone()).collect());
            let use_with = all.len() > fields.len();
            E::StructLit {
                name: name.name.clone(),
                fields: fields
                    .iter()
                    .map(|f| {
                        (
                            f.name.name.clone(),
                            lower_expr(
                                &f.value,
                                module,
                                osig,
                                typed,
                                ownership,
                                errors,
                                locals,
                                struct_fields,
                                fn_modules,
                            ),
                        )
                    })
                    .collect(),
                all_fields: all,
                use_with,
                ty: CirTy::Named {
                    name: name.name.clone(),
                    args: vec![],
                },
                span: expr.span,
            }
        }
        ExprKind::Throw(inner) => E::Throw {
            payload: Box::new(lower_expr(
                inner,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
            )),
            span: expr.span,
        },
        ExprKind::Try(inner) => E::Try {
            expr: Box::new(lower_expr(
                inner,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
            )),
            span: expr.span,
        },
        ExprKind::Catch { body: inner, arms } => {
            let lowered = lower_expr(
                inner,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
            );
            let catch_arms: Vec<CirCatchArm> = arms
                .iter()
                .map(|a| CirCatchArm {
                    wildcard: matches!(a.pat.kind, PatKind::Wildcard),
                    body: lower_expr(
                        &a.body,
                        module,
                        osig,
                        typed,
                        ownership,
                        errors,
                        locals,
                        struct_fields,
                        fn_modules,
                    ),
                    span: a.span,
                })
                .collect();
            E::Catch {
                expr: Box::new(lowered),
                arms: catch_arms,
                ty: CirTy::Error,
                span: expr.span,
            }
        }
        ExprKind::Block(b) => E::Block(lower_block(
            b,
            module,
            osig,
            typed,
            ownership,
            errors,
            &mut locals.clone(),
            false,
            struct_fields,
            fn_modules,
        )),
        _ => E::Unit { span: expr.span },
    }
}

fn infer_expr_ty(
    expr: &Expr,
    locals: &BTreeMap<String, CirTy>,
    typed: &TypedCrate,
    module: &str,
) -> CirTy {
    match &expr.kind {
        ExprKind::Ident(id) => locals.get(&id.name).cloned().unwrap_or(CirTy::Error),
        ExprKind::Str(_) => CirTy::Str,
        ExprKind::Int(_) => CirTy::Int,
        ExprKind::Call { func, .. } => {
            if let ExprKind::Ident(id) = &func.kind {
                let key = format!("{module}::{}", id.name);
                typed
                    .signatures
                    .get(&key)
                    .map(|s| CirTy::from_ty(&s.ret))
                    .unwrap_or(CirTy::Error)
            } else {
                CirTy::Error
            }
        }
        _ => CirTy::Error,
    }
}

fn should_clone_at_bind(
    osig: &crisp_ownership::OwnershipSignature,
    binding: &str,
    value: &Expr,
) -> bool {
    if !osig
        .applied_fallbacks
        .iter()
        .any(|f| f.kind == FallbackKind::CloneAtMove)
    {
        return false;
    }
    if let ExprKind::Ident(src) = &value.kind {
        return osig.auto_clones.iter().any(|ac| ac.binding == src.name);
    }
    let _ = binding;
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_hello_cir() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
        let cir = CirBuilder::build_crate(&root).expect("build");
        assert_eq!(cir.package_name, "hello");
        let main_mod = cir.modules.iter().find(|m| m.path == "main").expect("main");
        let fns: Vec<_> = main_mod
            .items
            .iter()
            .filter_map(|i| match i {
                CirItem::Function(f) => Some(f.name.as_str()),
                _ => None,
            })
            .collect();
        assert!(fns.contains(&"greet"));
        assert!(fns.contains(&"main"));
    }

    #[test]
    fn build_server_has_config_with_fn() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/server");
        let cir = CirBuilder::build_crate(&root).expect("build");
        let config = cir.modules.iter().find(|m| m.path == "config").expect("config");
        let st = config
            .items
            .iter()
            .find_map(|i| match i {
                CirItem::Struct(s) => Some(s),
                _ => None,
            })
            .expect("Config struct");
        assert!(st.with_fn.is_some());
        assert_eq!(st.fields.len(), 3);
    }

    #[test]
    fn build_fallible_marks_fallible_calls() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fallible");
        let cir = CirBuilder::build_crate(&root).expect("build");
        let main_mod = cir.modules.iter().find(|m| m.path == "main").expect("main");
        let read_config = main_mod.items.iter().find_map(|i| match i {
            CirItem::Function(f) if f.name == "read_config" => Some(f),
            _ => None,
        });
        assert!(read_config.is_some_and(|f| f.fallible));
    }
}
