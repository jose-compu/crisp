use crate::error::ErrorPassError;
use crate::result::{CrispErrorEnum, CrispErrorVariant, ErrorResult, ErrorSet, ErrorSig};
use crate::set::{absorbs_all, catch_handled_set, declared_set_from_fn, thrown_error_name};
use crisp_ast::expr::{Block, Expr, ExprKind, Stmt};
use crisp_ast::item::{FunctionDef, Item};
use crisp_resolve::ResolvedRustImport;
use crisp_resolve::module::load_module_graph;
use crisp_typeck::{TypeChecker, rust_import_returns_result};
use std::collections::BTreeMap;
use std::path::Path;

pub struct ErrorPass;

impl ErrorPass {
    pub fn analyze_crate(crate_root: &Path) -> Result<ErrorResult, ErrorPassError> {
        let typed = TypeChecker::check_crate(crate_root)?;
        let graph = load_module_graph(crate_root)?;
        let rust_imports = &typed.rust_imports;

        let mut fn_defs: BTreeMap<String, (String, FunctionDef)> = BTreeMap::new();
        for node in graph.modules.values() {
            for item in &node.ast.items {
                match item {
                    Item::Function(f) => {
                        let key = format!("{}::{}", node.module_path, f.name.name);
                        fn_defs.insert(key, (node.module_path.clone(), f.clone()));
                    }
                    Item::Impl(ib) => {
                        let ty_name = match &ib.ty.kind {
                            crisp_ast::ty::TypeKind::Named(id) => id.name.clone(),
                            _ => continue,
                        };
                        for f in &ib.items {
                            let key = format!("{}::{ty_name}::{}", node.module_path, f.name.name);
                            fn_defs.insert(key, (node.module_path.clone(), f.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut sigs: BTreeMap<String, ErrorSet> = BTreeMap::new();
        for key in fn_defs.keys() {
            sigs.insert(key.clone(), ErrorSet::new());
        }

        let max_iters = fn_defs.len().max(1) * 4 + 8;
        for _ in 0..max_iters {
            let mut changed = false;
            for (key, (module, def)) in &fn_defs {
                let local = collect_local_errors(module, def, &fn_defs, &sigs, rust_imports);
                let prev = sigs.get(key).cloned().unwrap_or_default();
                if prev != local {
                    changed = true;
                    sigs.insert(key.clone(), local);
                }
            }
            if !changed {
                break;
            }
        }

        let mut signatures = BTreeMap::new();
        let mut global = ErrorSet::new();

        for (key, (module, def)) in &fn_defs {
            let errors = sigs.get(key).cloned().unwrap_or_default();
            let (declared, asserts_never) = declared_set_from_fn(def);

            if asserts_never && !errors.is_empty() {
                return Err(ErrorPassError::NeverViolated {
                    name: def.name.name.clone(),
                    produced: format_error_set(&errors),
                    span: def.span,
                });
            }

            if let Some(ref decl) = declared
                && !decl.is_empty()
            {
                for e in errors.iter() {
                    if !decl.contains(e) {
                        return Err(ErrorPassError::DeclaredMismatch {
                            name: def.name.name.clone(),
                            declared: format_error_set(decl),
                            produced: e.clone(),
                            span: def.error_type.as_ref().map(|t| t.span).unwrap_or(def.span),
                        });
                    }
                }
            }

            let fallible = !errors.is_empty();
            global.extend(&errors);
            signatures.insert(
                key.clone(),
                ErrorSig {
                    module: module.clone(),
                    name: def.name.name.clone(),
                    fallible,
                    errors: errors.clone(),
                    declared,
                    asserts_never,
                    span: def.span,
                },
            );
        }

        Ok(ErrorResult {
            signatures,
            crisp_error: synthesize_enum(&global),
        })
    }
}

fn format_error_set(set: &ErrorSet) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(" | ")
}

fn synthesize_enum(global: &ErrorSet) -> CrispErrorEnum {
    let mut variants: Vec<CrispErrorVariant> = global
        .iter()
        .map(|name| CrispErrorVariant {
            name: name.clone(),
            payload_type: if name == "Thrown" {
                "String".into()
            } else {
                name.clone()
            },
        })
        .collect();
    variants.sort_by(|a, b| a.name.cmp(&b.name));
    CrispErrorEnum { variants }
}

fn collect_local_errors(
    module: &str,
    def: &FunctionDef,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    callee_sigs: &BTreeMap<String, ErrorSet>,
    rust_imports: &[ResolvedRustImport],
) -> ErrorSet {
    let mut out = ErrorSet::new();
    walk_expr(
        module,
        &def.body,
        fn_defs,
        callee_sigs,
        rust_imports,
        &mut out,
    );
    out
}

fn walk_block(
    module: &str,
    block: &Block,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    callee_sigs: &BTreeMap<String, ErrorSet>,
    rust_imports: &[ResolvedRustImport],
    out: &mut ErrorSet,
) {
    for stmt in &block.stmts {
        walk_stmt(module, stmt, fn_defs, callee_sigs, rust_imports, out);
    }
    if let Some(tail) = &block.tail {
        walk_expr(module, tail, fn_defs, callee_sigs, rust_imports, out);
    }
}

fn walk_stmt(
    module: &str,
    stmt: &Stmt,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    callee_sigs: &BTreeMap<String, ErrorSet>,
    rust_imports: &[ResolvedRustImport],
    out: &mut ErrorSet,
) {
    match stmt {
        Stmt::Expr(e) => walk_expr(module, e, fn_defs, callee_sigs, rust_imports, out),
        Stmt::Bind { value, .. } | Stmt::Assign { value, .. } => {
            walk_expr(module, value, fn_defs, callee_sigs, rust_imports, out);
        }
    }
}

fn walk_expr(
    module: &str,
    expr: &Expr,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    callee_sigs: &BTreeMap<String, ErrorSet>,
    rust_imports: &[ResolvedRustImport],
    out: &mut ErrorSet,
) {
    match &expr.kind {
        ExprKind::Block(b) => walk_block(module, b, fn_defs, callee_sigs, rust_imports, out),
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            walk_expr(module, cond, fn_defs, callee_sigs, rust_imports, out);
            walk_expr(module, then_branch, fn_defs, callee_sigs, rust_imports, out);
            if let Some(e) = else_branch {
                walk_expr(module, e, fn_defs, callee_sigs, rust_imports, out);
            }
        }
        ExprKind::Throw(inner) => {
            if let Some(name) = thrown_error_name(inner) {
                out.insert(name);
            }
        }
        ExprKind::Try(inner) => {
            walk_expr(module, inner, fn_defs, callee_sigs, rust_imports, out);
            propagate_call_errors(module, inner, fn_defs, callee_sigs, out);
            propagate_rust_import_errors(inner, rust_imports, out);
        }
        ExprKind::Catch { body, arms } => {
            let mut inner = ErrorSet::new();
            walk_expr(module, body, fn_defs, callee_sigs, rust_imports, &mut inner);
            let handled = catch_handled_set(arms);
            if absorbs_all(&handled) {
                // all errors from body absorbed
            } else {
                let remaining = ErrorSet::subtract(&inner, &handled);
                out.extend(&remaining);
            }
            for arm in arms {
                walk_expr(module, &arm.body, fn_defs, callee_sigs, rust_imports, out);
            }
        }
        ExprKind::Call { func, args } => {
            walk_expr(module, func, fn_defs, callee_sigs, rust_imports, out);
            for arg in args {
                walk_expr(module, arg, fn_defs, callee_sigs, rust_imports, out);
            }
            propagate_call_errors(module, func, fn_defs, callee_sigs, out);
            propagate_rust_import_errors(func, rust_imports, out);
        }
        ExprKind::MethodCall { receiver, args, .. } => {
            walk_expr(module, receiver, fn_defs, callee_sigs, rust_imports, out);
            for arg in args {
                walk_expr(module, arg, fn_defs, callee_sigs, rust_imports, out);
            }
        }
        ExprKind::Bind { value, .. } => {
            walk_expr(module, value, fn_defs, callee_sigs, rust_imports, out)
        }
        ExprKind::Assign { value, .. } => {
            walk_expr(module, value, fn_defs, callee_sigs, rust_imports, out)
        }
        ExprKind::Return(Some(v)) => walk_expr(module, v, fn_defs, callee_sigs, rust_imports, out),
        ExprKind::Binary { left, right, .. } => {
            walk_expr(module, left, fn_defs, callee_sigs, rust_imports, out);
            walk_expr(module, right, fn_defs, callee_sigs, rust_imports, out);
        }
        ExprKind::Unary { expr: inner, .. } => {
            walk_expr(module, inner, fn_defs, callee_sigs, rust_imports, out)
        }
        ExprKind::Field { base, .. } => {
            walk_expr(module, base, fn_defs, callee_sigs, rust_imports, out)
        }
        ExprKind::Pipe { left, right } => {
            walk_expr(module, left, fn_defs, callee_sigs, rust_imports, out);
            walk_expr(module, right, fn_defs, callee_sigs, rust_imports, out);
        }
        ExprKind::StructLit { fields, .. } => {
            for f in fields {
                walk_expr(module, &f.value, fn_defs, callee_sigs, rust_imports, out);
            }
        }
        ExprKind::Str(parts) => {
            for part in &parts.0 {
                if let crisp_ast::expr::StringPart::Expr(e) = part {
                    walk_expr(module, e, fn_defs, callee_sigs, rust_imports, out);
                }
            }
        }
        ExprKind::While { cond, body } => {
            walk_expr(module, cond, fn_defs, callee_sigs, rust_imports, out);
            walk_expr(module, body, fn_defs, callee_sigs, rust_imports, out);
        }
        ExprKind::For { iter, body, .. } => {
            walk_expr(module, iter, fn_defs, callee_sigs, rust_imports, out);
            walk_expr(module, body, fn_defs, callee_sigs, rust_imports, out);
        }
        ExprKind::Loop(body)
        | ExprKind::Async(body)
        | ExprKind::Await(body)
        | ExprKind::Spawn(body)
        | ExprKind::Unsafe(body) => {
            walk_expr(module, body, fn_defs, callee_sigs, rust_imports, out)
        }
        ExprKind::Break(Some(v)) => walk_expr(module, v, fn_defs, callee_sigs, rust_imports, out),
        ExprKind::Break(None) | ExprKind::Continue => {}
        ExprKind::Lambda { body, .. } => {
            walk_expr(module, body, fn_defs, callee_sigs, rust_imports, out)
        }
        _ => {}
    }
}

fn propagate_rust_import_errors(
    func: &Expr,
    rust_imports: &[ResolvedRustImport],
    out: &mut ErrorSet,
) {
    let ExprKind::Ident(id) = &func.kind else {
        return;
    };
    for imp in rust_imports {
        if imp.local_name == id.name && rust_import_returns_result(&imp.crate_name, &imp.item) {
            out.insert("Thrown");
            return;
        }
    }
}

fn propagate_call_errors(
    module: &str,
    func: &Expr,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    callee_sigs: &BTreeMap<String, ErrorSet>,
    out: &mut ErrorSet,
) {
    let Some(callee_key) = resolve_callee_key(module, func, fn_defs) else {
        return;
    };
    if let Some(errors) = callee_sigs.get(&callee_key) {
        out.extend(errors);
    }
}

fn resolve_callee_key(
    module: &str,
    func: &Expr,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
) -> Option<String> {
    match &func.kind {
        ExprKind::Ident(id) => {
            let local = format!("{module}::{}", id.name);
            if fn_defs.contains_key(&local) {
                return Some(local);
            }
            for (key, (m, def)) in fn_defs {
                if def.name.name == id.name {
                    return Some(key.clone());
                }
                if m != module && def.is_pub && def.name.name == id.name {
                    return Some(key.clone());
                }
            }
            None
        }
        ExprKind::Field { base, field } => {
            if let ExprKind::Ident(id) = &base.kind {
                let local = format!("{module}::{}::{}", id.name, field.name);
                if fn_defs.contains_key(&local) {
                    return Some(local);
                }
                let suffix = format!("::{}::{}", id.name, field.name);
                for key in fn_defs.keys() {
                    if key.ends_with(&suffix) {
                        return Some(key.clone());
                    }
                }
            }
            let suffix = format!("::{}", field.name);
            let hits: Vec<&String> = fn_defs
                .keys()
                .filter(|k| k.ends_with(&suffix) && k.matches("::").count() >= 2)
                .collect();
            if hits.len() == 1 {
                return Some(hits[0].clone());
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}"))
    }

    fn examples(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
    }

    #[test]
    fn infer_fallible_chain() {
        let result = ErrorPass::analyze_crate(&fixture("fallible")).expect("fallible");
        let read = result.get("main", "read_config").expect("read_config");
        assert!(read.fallible);
        assert!(read.errors.contains("IoError"));
        assert!(read.errors.contains("ParseError"));
    }

    #[test]
    fn catch_makes_main_infallible() {
        let result = ErrorPass::analyze_crate(&fixture("fallible")).expect("fallible");
        let main = result.get("main", "main").expect("main");
        assert!(!main.fallible);
    }

    #[test]
    fn synthesize_crisp_error_enum() {
        let result = ErrorPass::analyze_crate(&fixture("fallible")).expect("fallible");
        let names: Vec<_> = result
            .crisp_error
            .variants
            .iter()
            .map(|v| v.name.as_str())
            .collect();
        assert!(names.contains(&"IoError"));
        assert!(names.contains(&"ParseError"));
    }

    #[test]
    fn never_annotation_rejected() {
        let err = ErrorPass::analyze_crate(&fixture("never_bad")).expect_err("never");
        assert!(matches!(err, ErrorPassError::NeverViolated { .. }));
    }

    #[test]
    fn declared_set_rejected() {
        let err = ErrorPass::analyze_crate(&fixture("declared_bad")).expect_err("declared");
        assert!(matches!(err, ErrorPassError::DeclaredMismatch { .. }));
    }

    #[test]
    fn hello_has_no_errors() {
        let result = ErrorPass::analyze_crate(&examples("hello")).expect("hello");
        assert!(result.signatures.values().all(|s| !s.fallible));
    }

    #[test]
    fn rust_import_marks_main_fallible() {
        let result = ErrorPass::analyze_crate(&examples("rust_import")).expect("rust_import");
        let main = result.get("main", "main").expect("main");
        assert!(main.fallible, "Result APIs should mark main fallible");
        assert!(main.errors.contains("Thrown"));
        assert!(
            result
                .crisp_error
                .variants
                .iter()
                .any(|v| v.name == "Thrown")
        );
    }
}
