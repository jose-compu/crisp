//! Lower analyzed AST to CIR (spec §17.1).

use crate::node::*;
use crate::source_map::SourceMap;
use crate::synthesize::{lower_enum, lower_struct, synthesize_shape_trait};
use crate::ty::CirTy;
use crisp_ast::expr::{BinaryOp, Block, Expr, ExprKind, Stmt, StringPart};
use crisp_ast::item::{ExternBlock, FunctionDef, Item, TypeBody};
use crisp_ast::pat::{Pat, PatKind};
use crisp_errors::{ErrorPass, ErrorResult};
use crisp_ownership::{FallbackKind, OwnershipMode, OwnershipPass, OwnershipResult};
use crisp_regions::{RegionPass, RegionResult};
use crisp_resolve::module::{ModuleGraph, load_module_graph};
use crisp_resolve::stdlib::stdlib_fn_modules;
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
                if let Item::TypeDef(td) = ast_item
                    && let TypeBody::Struct(_) = &td.body
                {
                    let field_types = struct_types.get(&td.name.name);
                    let st = lower_struct(td, field_types.unwrap_or(&BTreeMap::new()));
                    all_structs.push((node.module_path.clone(), st));
                }
            }
        }
        let mut fn_modules = collect_fn_modules(graph);
        fn_modules.extend(stdlib_fn_modules());
        for imp in &typed.rust_imports {
            fn_modules.insert(imp.local_name.clone(), format!("rust.{}", imp.crate_name));
        }
        let extern_fns = collect_extern_fns(graph);
        let struct_fields = collect_struct_field_names(&all_structs);
        // Pre-collect shapes so field access can lower to accessor MethodCalls (§3.5).
        let mut shape_traits = Vec::new();
        for node in graph.modules.values() {
            for ast_item in &node.ast.items {
                if let Item::ShapeDef(shape) = ast_item {
                    shape_traits.push(synthesize_shape_trait(shape, &all_structs));
                }
            }
        }
        let shape_fields: BTreeMap<String, Vec<String>> = shape_traits
            .iter()
            .map(|s| {
                (
                    s.name.clone(),
                    s.fields.iter().map(|(n, _)| n.clone()).collect(),
                )
            })
            .collect();
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
                    Item::ShapeDef(_) => {
                        // Already synthesized into `shape_traits` above.
                    }
                    Item::TraitDef(td) => {
                        items.push(CirItem::Trait(lower_trait_def(td)));
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
                                &extern_fns,
                                &shape_fields,
                            )));
                        }
                    }
                    Item::Impl(ib) => {
                        let ty_name = type_name_from_ast(&ib.ty);
                        let mut fns = Vec::new();
                        for f in &ib.items {
                            let key = format!("{}::{ty_name}::{}", node.module_path, f.name.name);
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
                                    &extern_fns,
                                    &shape_fields,
                                ));
                            }
                        }
                        let trait_name = ib.trait_name.as_ref().map(|i| i.name.clone());
                        let trait_args = if !ib.trait_args.is_empty() {
                            ib.trait_args.iter().map(ast_type_to_cir_ty).collect()
                        } else if let Some(tn) = &trait_name {
                            let key = format!("{}::{tn} for {ty_name}", node.module_path);
                            typed
                                .impl_trait_args
                                .get(&key)
                                .map(|tys| tys.iter().map(CirTy::from_ty).collect())
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        items.push(CirItem::Impl(CirImpl {
                            trait_name,
                            trait_args,
                            ty_name,
                            functions: fns,
                            span: ib.span,
                        }));
                    }
                    Item::Extern(ext) => {
                        items.push(CirItem::Extern(lower_extern(ext)));
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

fn lower_trait_def(td: &crisp_ast::item::TraitDef) -> CirTrait {
    use crisp_ast::expr::Param;
    CirTrait {
        name: td.name.name.clone(),
        generics: td.generics.iter().map(|g| g.name.clone()).collect(),
        methods: td
            .items
            .iter()
            .map(|m| CirTraitMethod {
                name: m.name.name.clone(),
                params: m
                    .params
                    .iter()
                    .map(|p: &Param| {
                        let ty = if p.name.name == "self" && p.ty.is_none() {
                            CirTy::Named {
                                name: "Self".into(),
                                args: vec![],
                            }
                        } else {
                            p.ty.as_ref()
                                .map(ast_type_to_cir_ty)
                                .unwrap_or(CirTy::Error)
                        };
                        (p.name.name.clone(), ty)
                    })
                    .collect(),
                ret: m
                    .ret_type
                    .as_ref()
                    .map(ast_type_to_cir_ty)
                    .unwrap_or(CirTy::Unit),
                default_body: m.default_body.as_ref().map(lower_trait_default_expr),
            })
            .collect(),
        span: td.span,
    }
}

/// Lower trait default bodies for literal / simple expressions (§3.6 / #59).
fn lower_trait_default_expr(expr: &Expr) -> CirExpr {
    match &expr.kind {
        ExprKind::Str(parts) => {
            let mut s = String::new();
            for p in &parts.0 {
                if let StringPart::Lit(l) = p {
                    s.push_str(l);
                }
            }
            CirExpr::Str {
                value: s,
                span: expr.span,
            }
        }
        ExprKind::Int(n) => CirExpr::Int {
            value: *n,
            span: expr.span,
        },
        ExprKind::Float(f) => CirExpr::Float {
            value: *f,
            span: expr.span,
        },
        ExprKind::Bool(b) => CirExpr::Ident {
            name: if *b { "true" } else { "false" }.into(),
            ty: CirTy::Bool,
            span: expr.span,
        },
        ExprKind::Block(b) if b.stmts.is_empty() => b
            .tail
            .as_ref()
            .map(|t| lower_trait_default_expr(t))
            .unwrap_or(CirExpr::Unit { span: expr.span }),
        _ => CirExpr::Unit { span: expr.span },
    }
}

fn type_extra_bounds(ty: &crisp_ast::ty::Type) -> Vec<String> {
    use crisp_ast::ty::{TypeBound, TypeKind};
    match &ty.kind {
        TypeKind::Constrained { inner, bounds } => {
            let mut out = type_extra_bounds(inner);
            for b in bounds {
                match b {
                    TypeBound::Shape(id) | TypeBound::Trait(id) => out.push(id.name.clone()),
                }
            }
            out
        }
        _ => Vec::new(),
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
            "char" => CirTy::Char,
            other => CirTy::Named {
                name: other.to_string(),
                args: vec![],
            },
        },
        TypeKind::Generic { base, args } => {
            let mut cir = ast_type_to_cir_ty(base);
            if let CirTy::Named {
                args: ref mut a, ..
            } = cir
            {
                *a = args.iter().map(ast_type_to_cir_ty).collect();
            }
            cir
        }
        TypeKind::Option(inner) => CirTy::Option(Box::new(ast_type_to_cir_ty(inner))),
        TypeKind::Tuple(ts) => CirTy::Tuple(ts.iter().map(ast_type_to_cir_ty).collect()),
        TypeKind::Ref { mutable, inner } => CirTy::Ref {
            mutable: *mutable,
            inner: Box::new(ast_type_to_cir_ty(inner)),
        },
        TypeKind::Never => CirTy::Never,
        TypeKind::Unit => CirTy::Unit,
        TypeKind::Constrained { inner, .. } => ast_type_to_cir_ty(inner),
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

fn collect_struct_field_names(structs: &[(String, CirStruct)]) -> BTreeMap<String, Vec<String>> {
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

#[derive(Clone, Copy)]
struct LowerCtx<'a> {
    propagate_errors: bool,
    extern_fns: &'a std::collections::BTreeSet<String>,
    shape_fields: &'a BTreeMap<String, Vec<String>>,
}

impl<'a> LowerCtx<'a> {
    fn new(
        extern_fns: &'a std::collections::BTreeSet<String>,
        shape_fields: &'a BTreeMap<String, Vec<String>>,
    ) -> Self {
        Self {
            propagate_errors: true,
            extern_fns,
            shape_fields,
        }
    }
}

fn expr_is_async(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Async(_) | ExprKind::Await(_) => true,
        ExprKind::Block(b) => {
            b.stmts.iter().any(|s| match s {
                Stmt::Expr(e) => expr_is_async(e),
                Stmt::Bind { value, .. } => expr_is_async(value),
                Stmt::Assign { value, .. } => expr_is_async(value),
            }) || b.tail.as_ref().is_some_and(|t| expr_is_async(t))
        }
        _ => false,
    }
}

fn lower_extern(ext: &ExternBlock) -> CirExternBlock {
    CirExternBlock {
        abi: ext.abi.clone(),
        functions: ext
            .functions
            .iter()
            .map(|f| CirExternFn {
                name: f.name.name.clone(),
                params: f
                    .params
                    .iter()
                    .map(|p| CirParam {
                        name: p.name.name.clone(),
                        ty: p
                            .ty
                            .as_ref()
                            .map(ast_type_to_cir_ty)
                            .unwrap_or(CirTy::Error),
                        mode: OwnershipMode::Owned,
                        lifetime: None,
                        extra_bounds: p.ty.as_ref().map(type_extra_bounds).unwrap_or_default(),
                        span: p.span,
                    })
                    .collect(),
                ret: f
                    .ret_type
                    .as_ref()
                    .map(ast_type_to_cir_ty)
                    .unwrap_or(CirTy::Unit),
                span: f.span,
            })
            .collect(),
        span: ext.span,
    }
}

fn lower_pat(pat: &Pat) -> CirPat {
    use crate::node::CirPat as P;
    match &pat.kind {
        PatKind::Wildcard => P::Wildcard { span: pat.span },
        PatKind::Ident(id) => P::Ident {
            name: id.name.clone(),
            span: pat.span,
        },
        PatKind::Literal(expr) => {
            if let ExprKind::Int(n) = &expr.kind {
                P::Int {
                    value: *n,
                    span: pat.span,
                }
            } else {
                P::Wildcard { span: pat.span }
            }
        }
        PatKind::Struct { name, fields, .. } => P::Struct {
            name: name.name.clone(),
            fields: fields
                .iter()
                .map(|f| {
                    (
                        f.name.name.clone(),
                        f.pat
                            .as_ref()
                            .map(lower_pat)
                            .unwrap_or_else(|| P::Wildcard { span: f.span }),
                    )
                })
                .collect(),
            span: pat.span,
        },
        PatKind::Enum {
            name,
            variant,
            args,
        } => P::Enum {
            ty_name: name.name.clone(),
            variant: variant.name.clone(),
            args: args.iter().map(lower_pat).collect(),
            span: pat.span,
        },
        _ => P::Wildcard { span: pat.span },
    }
}

fn print_arg_is_debug(expr: &CirExpr) -> bool {
    !matches!(
        expr,
        CirExpr::Str { .. } | CirExpr::Int { .. } | CirExpr::Float { .. } | CirExpr::Format { .. }
    )
}

fn collect_extern_fns(graph: &ModuleGraph) -> std::collections::BTreeSet<String> {
    let mut set = std::collections::BTreeSet::new();
    for node in graph.modules.values() {
        for item in &node.ast.items {
            if let Item::Extern(ext) = item {
                for f in &ext.functions {
                    set.insert(f.name.name.clone());
                }
            }
        }
    }
    set
}

#[allow(clippy::too_many_arguments)]
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
    extern_fns: &std::collections::BTreeSet<String>,
    shape_fields: &BTreeMap<String, Vec<String>>,
) -> CirFunction {
    let (emit_params, emit_ret, emit_gens) = tsig.emit_view();
    let params: Vec<CirParam> = osig
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, mode))| {
            let ty = emit_params
                .get(i)
                .map(|(_, t)| CirTy::from_ty(t))
                .unwrap_or(CirTy::Error);
            let lifetime = lt.and_then(|l| l.param_lifetimes.get(i).cloned().flatten());
            CirParam {
                name: name.clone(),
                ty,
                mode: *mode,
                lifetime,
                extra_bounds: def
                    .params
                    .get(i)
                    .and_then(|p| p.ty.as_ref())
                    .map(type_extra_bounds)
                    .unwrap_or_default(),
                span: def.span,
            }
        })
        .collect();

    let ret = if esig.fallible {
        CirTy::Result {
            ok: Box::new(CirTy::from_ty(&emit_ret)),
            err: "CrispError".into(),
        }
    } else {
        CirTy::from_ty(&emit_ret)
    };

    let mut locals: BTreeMap<String, CirTy> = BTreeMap::new();
    for (name, t) in &emit_params {
        locals.insert(name.clone(), CirTy::from_ty(t));
    }

    let is_async = expr_is_async(&def.body);
    let body_src = if let ExprKind::Async(inner) = &def.body.kind {
        inner.as_ref()
    } else {
        &def.body
    };

    let body = lower_body(
        body_src,
        module,
        osig,
        typed,
        ownership,
        errors,
        &mut locals,
        esig.fallible,
        struct_fields,
        fn_modules,
        extern_fns,
        shape_fields,
    );

    CirFunction {
        name: def.name.name.clone(),
        is_pub: def.is_pub,
        is_main: def.name.name == "main",
        is_async,
        generics: if tsig.mono_args.is_some() {
            Vec::new()
        } else if def.generics.is_empty() {
            emit_gens
        } else {
            def.generics.iter().map(|g| g.name.clone()).collect()
        },
        params,
        ret,
        fallible: esig.fallible,
        lifetimes: lt.cloned(),
        body,
        span: def.span,
    }
}

#[allow(clippy::too_many_arguments)]
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
    extern_fns: &std::collections::BTreeSet<String>,
    shape_fields: &BTreeMap<String, Vec<String>>,
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
            extern_fns,
            shape_fields,
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
                LowerCtx::new(extern_fns, shape_fields),
            );
            CirBlock {
                stmts: vec![],
                tail: Some(Box::new(tail)),
                span: expr.span,
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
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
    extern_fns: &std::collections::BTreeSet<String>,
    shape_fields: &BTreeMap<String, Vec<String>>,
) -> CirBlock {
    let mut stmts = Vec::new();
    for stmt in &block.stmts {
        match stmt {
            Stmt::Bind {
                pat,
                mutable,
                value,
                ..
            } => {
                if let PatKind::Ident(name) = &pat.kind {
                    let mut lowered = lower_expr(
                        value,
                        module,
                        osig,
                        typed,
                        ownership,
                        errors,
                        locals,
                        struct_fields,
                        fn_modules,
                        LowerCtx::new(extern_fns, shape_fields),
                    );
                    if should_clone_at_bind(osig, &name.name, value) {
                        lowered = CirExpr::Clone {
                            expr: Box::new(lowered),
                            span: value.span,
                        };
                    }
                    let ty = cir_expr_value_ty(&lowered)
                        .unwrap_or_else(|| infer_expr_ty(value, locals, typed, module));
                    locals.insert(name.name.clone(), ty);
                    stmts.push(CirStmt::Let {
                        name: name.name.clone(),
                        mutable: *mutable,
                        value: lowered,
                        span: value.span,
                    });
                }
            }
            Stmt::Expr(e) => {
                stmts.push(CirStmt::Expr(lower_expr(
                    e,
                    module,
                    osig,
                    typed,
                    ownership,
                    errors,
                    locals,
                    struct_fields,
                    fn_modules,
                    LowerCtx::new(extern_fns, shape_fields),
                )));
            }
            Stmt::Assign { target, value, .. } => {
                stmts.push(CirStmt::Assign {
                    target: target.name.clone(),
                    value: lower_expr(
                        value,
                        module,
                        osig,
                        typed,
                        ownership,
                        errors,
                        locals,
                        struct_fields,
                        fn_modules,
                        LowerCtx::new(extern_fns, shape_fields),
                    ),
                    span: value.span,
                });
            }
        }
    }
    let tail = block.tail.as_ref().map(|e| {
        lower_expr(
            e,
            module,
            osig,
            typed,
            ownership,
            errors,
            locals,
            struct_fields,
            fn_modules,
            LowerCtx::new(extern_fns, shape_fields),
        )
    });
    let _ = fn_fallible;
    CirBlock {
        stmts,
        tail: tail.map(Box::new),
        span: block.span,
    }
}

#[allow(clippy::too_many_arguments)]
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
    ctx: LowerCtx,
) -> CirExpr {
    use crate::node::{CirExpr as E, CirFormatPart};
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
        ExprKind::Bool(b) => E::Bool {
            value: *b,
            span: expr.span,
        },
        ExprKind::Float(f) => E::Float {
            value: *f,
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
                                e,
                                module,
                                osig,
                                typed,
                                ownership,
                                errors,
                                locals,
                                struct_fields,
                                fn_modules,
                                ctx,
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
        ExprKind::Field { base, field } => {
            // Unit enum variant: Color.Red
            if let ExprKind::Ident(id) = &base.kind
                && looks_like_type_name(&id.name)
                && !locals.contains_key(&id.name)
            {
                return E::EnumVariant {
                    ty_name: id.name.clone(),
                    variant: field.name.clone(),
                    args: vec![],
                    ty: CirTy::Named {
                        name: id.name.clone(),
                        args: vec![],
                    },
                    span: expr.span,
                };
            }
            let lowered_base = lower_expr(
                base,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            );
            // Shape-typed receivers: field access → accessor method (§3.5 / #61).
            if let Some(ty_name) = cir_expr_named_ty(&lowered_base)
                && ctx.shape_fields.contains_key(&ty_name)
            {
                return E::MethodCall {
                    receiver: Box::new(lowered_base),
                    method: field.name.clone(),
                    args: vec![],
                    ty: CirTy::Error,
                    span: expr.span,
                };
            }
            let lowered = E::Field {
                base: Box::new(lowered_base),
                field: field.name.clone(),
                ty: CirTy::Error,
                span: expr.span,
            };
            // Borrowed params (`&T`) cannot move fields out; clone (issue #12).
            if field_access_needs_clone(osig, base) {
                E::Clone {
                    expr: Box::new(lowered),
                    span: expr.span,
                }
            } else {
                lowered
            }
        }
        ExprKind::Binary { op, left, right } => {
            let l = lower_expr(
                left,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            );
            let r = lower_expr(
                right,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            );
            let cir_op = match op {
                BinaryOp::Concat => CirBinOp::Concat,
                BinaryOp::Add => CirBinOp::Add,
                BinaryOp::Sub => CirBinOp::Sub,
                BinaryOp::Mul => CirBinOp::Mul,
                BinaryOp::Div => CirBinOp::Div,
                BinaryOp::Pow => CirBinOp::Pow,
                BinaryOp::Eq => CirBinOp::Eq,
                BinaryOp::Ne => CirBinOp::Ne,
                BinaryOp::Lt => CirBinOp::Lt,
                BinaryOp::Le => CirBinOp::Le,
                BinaryOp::Gt => CirBinOp::Gt,
                BinaryOp::Ge => CirBinOp::Ge,
                BinaryOp::And => CirBinOp::And,
                BinaryOp::Or => CirBinOp::Or,
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
            // Associated inherent method / enum variant: Type.foo(args)
            if let ExprKind::Field { base, field } = &func.kind
                && let ExprKind::Ident(id) = &base.kind
                && looks_like_type_name(&id.name)
                && !locals.contains_key(&id.name)
            {
                let call_args: Vec<CirExpr> = args
                    .iter()
                    .map(|arg| {
                        lower_expr(
                            arg,
                            module,
                            osig,
                            typed,
                            ownership,
                            errors,
                            locals,
                            struct_fields,
                            fn_modules,
                            ctx,
                        )
                    })
                    .collect();
                if let Some(key) = typed
                    .inherent_methods
                    .get(&id.name)
                    .and_then(|m| m.get(&field.name))
                {
                    let ret_ty = typed
                        .signatures
                        .get(key)
                        .map(|s| CirTy::from_ty(&s.ret))
                        .unwrap_or(CirTy::Error);
                    return E::AssocCall {
                        ty_name: id.name.clone(),
                        method: field.name.clone(),
                        args: call_args,
                        ty: ret_ty,
                        span: expr.span,
                    };
                }
                return E::EnumVariant {
                    ty_name: id.name.clone(),
                    variant: field.name.clone(),
                    args: call_args,
                    ty: CirTy::Named {
                        name: id.name.clone(),
                        args: vec![],
                    },
                    span: expr.span,
                };
            }
            // Instance method: recv.method(args)
            if let ExprKind::Field { base, field } = &func.kind {
                let receiver = lower_expr(
                    base,
                    module,
                    osig,
                    typed,
                    ownership,
                    errors,
                    locals,
                    struct_fields,
                    fn_modules,
                    ctx,
                );
                if let Some(ty_name) = cir_expr_named_ty(&receiver)
                    && let Some(key) = typed
                        .inherent_methods
                        .get(&ty_name)
                        .and_then(|m| m.get(&field.name))
                {
                    let ret_ty = typed
                        .signatures
                        .get(key)
                        .map(|s| CirTy::from_ty(&s.ret))
                        .unwrap_or(CirTy::Error);
                    let call_args: Vec<CirExpr> = args
                        .iter()
                        .map(|arg| {
                            lower_expr(
                                arg,
                                module,
                                osig,
                                typed,
                                ownership,
                                errors,
                                locals,
                                struct_fields,
                                fn_modules,
                                ctx,
                            )
                        })
                        .collect();
                    return E::MethodCall {
                        receiver: Box::new(receiver),
                        method: field.name.clone(),
                        args: call_args,
                        ty: ret_ty,
                        span: expr.span,
                    };
                }
            }
            if let ExprKind::Ident(id) = &func.kind {
                if (id.name == "print" || id.name == "log") && args.len() == 1 {
                    let arg = lower_expr(
                        &args[0],
                        module,
                        osig,
                        typed,
                        ownership,
                        errors,
                        locals,
                        struct_fields,
                        fn_modules,
                        ctx,
                    );
                    let debug = print_arg_is_debug(&arg);
                    return E::Print {
                        arg: Box::new(arg),
                        debug,
                        span: expr.span,
                    };
                }
                let callee_module = fn_modules
                    .get(&id.name)
                    .cloned()
                    .unwrap_or_else(|| module.to_string());
                let key = format!("{callee_module}::{}", id.name);
                let rust_result = callee_module
                    .strip_prefix("rust.")
                    .is_some_and(|crate_name| {
                        crisp_typeck::rust_import_returns_result(crate_name, &id.name)
                    });
                let fallible = rust_result
                    || errors
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
                            arg,
                            module,
                            osig,
                            typed,
                            ownership,
                            errors,
                            locals,
                            struct_fields,
                            fn_modules,
                            ctx,
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
                    propagate_error: fallible && ctx.propagate_errors,
                    is_extern: ctx.extern_fns.contains(&id.name),
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
                                ctx,
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
                ctx,
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
                ctx,
            )),
            span: expr.span,
        },
        ExprKind::Catch { body: inner, arms } => {
            let catch_ctx = LowerCtx {
                propagate_errors: false,
                extern_fns: ctx.extern_fns,
                shape_fields: ctx.shape_fields,
            };
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
                catch_ctx,
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
                        LowerCtx::new(ctx.extern_fns, ctx.shape_fields),
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
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => E::If {
            cond: Box::new(lower_expr(
                cond,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            )),
            then_branch: Box::new(lower_expr(
                then_branch,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            )),
            else_branch: else_branch.as_ref().map(|e| {
                Box::new(lower_expr(
                    e,
                    module,
                    osig,
                    typed,
                    ownership,
                    errors,
                    locals,
                    struct_fields,
                    fn_modules,
                    ctx,
                ))
            }),
            ty: CirTy::Error,
            span: expr.span,
        },
        ExprKind::Match { scrutinee, arms } => {
            let mut lowered_scrut = lower_expr(
                scrutinee,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            );
            // Match by value; clone when the scrutinee is a borrowed param.
            if let ExprKind::Ident(id) = &scrutinee.kind
                && osig.params.iter().any(|(n, m)| {
                    n == &id.name && matches!(m, OwnershipMode::Borrow | OwnershipMode::MutBorrow)
                })
            {
                lowered_scrut = E::Clone {
                    expr: Box::new(lowered_scrut),
                    span: scrutinee.span,
                };
            }
            E::Match {
                scrutinee: Box::new(lowered_scrut),
                arms: arms
                    .iter()
                    .map(|a| CirMatchArm {
                        pat: lower_pat(&a.pat),
                        guard: a.guard.as_ref().map(|g| {
                            lower_expr(
                                g,
                                module,
                                osig,
                                typed,
                                ownership,
                                errors,
                                locals,
                                struct_fields,
                                fn_modules,
                                ctx,
                            )
                        }),
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
                            ctx,
                        ),
                        span: a.span,
                    })
                    .collect(),
                ty: CirTy::Error,
                span: expr.span,
            }
        }
        ExprKind::While { cond, body } => E::While {
            cond: Box::new(lower_expr(
                cond,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            )),
            body: Box::new(lower_expr(
                body,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            )),
            span: expr.span,
        },
        ExprKind::For { pat, iter, body } => {
            let mut for_locals = locals.clone();
            if let PatKind::Ident(id) = &pat.kind {
                for_locals.insert(id.name.clone(), CirTy::Int);
            }
            E::For {
                pat: lower_pat(pat),
                iter: Box::new(lower_expr(
                    iter,
                    module,
                    osig,
                    typed,
                    ownership,
                    errors,
                    locals,
                    struct_fields,
                    fn_modules,
                    ctx,
                )),
                body: Box::new(lower_expr(
                    body,
                    module,
                    osig,
                    typed,
                    ownership,
                    errors,
                    &for_locals,
                    struct_fields,
                    fn_modules,
                    ctx,
                )),
                span: expr.span,
            }
        }
        ExprKind::Loop(body) => E::Loop {
            body: Box::new(lower_expr(
                body,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            )),
            span: expr.span,
        },
        ExprKind::Break(value) => E::Break {
            value: value.as_ref().map(|v| {
                Box::new(lower_expr(
                    v,
                    module,
                    osig,
                    typed,
                    ownership,
                    errors,
                    locals,
                    struct_fields,
                    fn_modules,
                    ctx,
                ))
            }),
            span: expr.span,
        },
        ExprKind::Continue => E::Continue { span: expr.span },
        ExprKind::Unsafe(inner) => E::Unsafe {
            body: Box::new(lower_expr(
                inner,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            )),
            span: expr.span,
        },
        ExprKind::Async(inner) => E::Async {
            body: Box::new(lower_expr(
                inner,
                module,
                osig,
                typed,
                ownership,
                errors,
                locals,
                struct_fields,
                fn_modules,
                ctx,
            )),
            span: expr.span,
        },
        ExprKind::Await(inner) => E::Await {
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
                ctx,
            )),
            ty: CirTy::Error,
            span: expr.span,
        },
        ExprKind::Spawn(inner) => E::Spawn {
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
                ctx,
            )),
            span: expr.span,
        },
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
            ctx.extern_fns,
            ctx.shape_fields,
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
        ExprKind::Float(_) => CirTy::Float,
        ExprKind::StructLit { name, .. } => CirTy::Named {
            name: name.name.clone(),
            args: vec![],
        },
        ExprKind::Call { func, .. } => {
            if let ExprKind::Field { base, field } = &func.kind {
                if let ExprKind::Ident(id) = &base.kind
                    && let Some(key) = typed
                        .inherent_methods
                        .get(&id.name)
                        .and_then(|m| m.get(&field.name))
                {
                    return typed
                        .signatures
                        .get(key)
                        .map(|s| CirTy::from_ty(&s.ret))
                        .unwrap_or(CirTy::Error);
                }
                if let ExprKind::Ident(recv) = &base.kind
                    && let Some(CirTy::Named { name, .. }) = locals.get(&recv.name)
                    && let Some(key) = typed
                        .inherent_methods
                        .get(name)
                        .and_then(|m| m.get(&field.name))
                {
                    return typed
                        .signatures
                        .get(key)
                        .map(|s| CirTy::from_ty(&s.ret))
                        .unwrap_or(CirTy::Error);
                }
            }
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

fn cir_expr_value_ty(expr: &CirExpr) -> Option<CirTy> {
    match expr {
        CirExpr::Ident { ty, .. }
        | CirExpr::Call { ty, .. }
        | CirExpr::Field { ty, .. }
        | CirExpr::AssocCall { ty, .. }
        | CirExpr::MethodCall { ty, .. }
        | CirExpr::EnumVariant { ty, .. }
        | CirExpr::StructLit { ty, .. }
        | CirExpr::BinOp { ty, .. }
        | CirExpr::If { ty, .. }
        | CirExpr::Match { ty, .. } => Some(ty.clone()),
        CirExpr::Int { .. } => Some(CirTy::Int),
        CirExpr::Float { .. } => Some(CirTy::Float),
        CirExpr::Bool { .. } => Some(CirTy::Bool),
        CirExpr::Str { .. } | CirExpr::Format { .. } => Some(CirTy::Str),
        CirExpr::Clone { expr, .. } | CirExpr::Borrow { expr, .. } => cir_expr_value_ty(expr),
        _ => None,
    }
}

fn looks_like_type_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

fn cir_expr_named_ty(expr: &CirExpr) -> Option<String> {
    match expr {
        CirExpr::Ident {
            ty: CirTy::Named { name, .. },
            ..
        }
        | CirExpr::Field {
            ty: CirTy::Named { name, .. },
            ..
        }
        | CirExpr::Call {
            ty: CirTy::Named { name, .. },
            ..
        }
        | CirExpr::AssocCall {
            ty: CirTy::Named { name, .. },
            ..
        }
        | CirExpr::MethodCall {
            ty: CirTy::Named { name, .. },
            ..
        }
        | CirExpr::EnumVariant {
            ty: CirTy::Named { name, .. },
            ..
        } => Some(name.clone()),
        CirExpr::StructLit { name, .. } => Some(name.clone()),
        CirExpr::Clone { expr, .. } | CirExpr::Borrow { expr, .. } => cir_expr_named_ty(expr),
        _ => None,
    }
}

fn field_access_needs_clone(osig: &crisp_ownership::OwnershipSignature, base: &Expr) -> bool {
    let ExprKind::Ident(id) = &base.kind else {
        return false;
    };
    osig.params.iter().any(|(name, mode)| {
        name == &id.name && matches!(mode, OwnershipMode::Borrow | OwnershipMode::MutBorrow)
    })
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
        let config = cir
            .modules
            .iter()
            .find(|m| m.path == "config")
            .expect("config");
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
