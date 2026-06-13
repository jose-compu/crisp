use crate::error::ErrorPassError;
use crate::result::{CrispErrorEnum, CrispErrorVariant, ErrorResult, ErrorSet, ErrorSig};
use crate::set::{
    absorbs_all, catch_handled_set, declared_set_from_fn, thrown_error_name,
};
use crisp_ast::expr::{Block, Expr, ExprKind, Stmt};
use crisp_ast::item::{FunctionDef, Item};
use crisp_resolve::module::load_module_graph;
use crisp_typeck::TypeChecker;
use std::collections::BTreeMap;
use std::path::Path;

pub struct ErrorPass;

impl ErrorPass {
    pub fn analyze_crate(crate_root: &Path) -> Result<ErrorResult, ErrorPassError> {
        let _ = TypeChecker::check_crate(crate_root)?;
        let graph = load_module_graph(crate_root)?;

        let mut fn_defs: BTreeMap<String, (String, FunctionDef)> = BTreeMap::new();
        for node in graph.modules.values() {
            for item in &node.ast.items {
                if let Item::Function(f) = item {
                    let key = format!("{}::{}", node.module_path, f.name.name);
                    fn_defs.insert(key, (node.module_path.clone(), f.clone()));
                }
            }
        }

        let mut sigs: BTreeMap<String, ErrorSet> = BTreeMap::new();
        for (key, _) in &fn_defs {
            sigs.insert(key.clone(), ErrorSet::new());
        }

        let max_iters = fn_defs.len().max(1) * 4 + 8;
        for _ in 0..max_iters {
            let mut changed = false;
            for (key, (module, def)) in &fn_defs {
                let local = collect_local_errors(module, def, &fn_defs, &sigs);
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

            if let Some(ref decl) = declared {
                if !decl.is_empty() {
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
) -> ErrorSet {
    let mut out = ErrorSet::new();
    walk_expr(module, &def.body, fn_defs, callee_sigs, &mut out);
    out
}

fn walk_block(
    module: &str,
    block: &Block,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    callee_sigs: &BTreeMap<String, ErrorSet>,
    out: &mut ErrorSet,
) {
    for stmt in &block.stmts {
        walk_stmt(module, stmt, fn_defs, callee_sigs, out);
    }
    if let Some(tail) = &block.tail {
        walk_expr(module, tail, fn_defs, callee_sigs, out);
    }
}

fn walk_stmt(
    module: &str,
    stmt: &Stmt,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    callee_sigs: &BTreeMap<String, ErrorSet>,
    out: &mut ErrorSet,
) {
    match stmt {
        Stmt::Expr(e) => walk_expr(module, e, fn_defs, callee_sigs, out),
        Stmt::Bind { value, .. } | Stmt::Assign { value, .. } => {
            walk_expr(module, value, fn_defs, callee_sigs, out);
        }
    }
}

fn walk_expr(
    module: &str,
    expr: &Expr,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    callee_sigs: &BTreeMap<String, ErrorSet>,
    out: &mut ErrorSet,
) {
    match &expr.kind {
        ExprKind::Block(b) => walk_block(module, b, fn_defs, callee_sigs, out),
        ExprKind::If { cond, then_branch, else_branch } => {
            walk_expr(module, cond, fn_defs, callee_sigs, out);
            walk_expr(module, then_branch, fn_defs, callee_sigs, out);
            if let Some(e) = else_branch {
                walk_expr(module, e, fn_defs, callee_sigs, out);
            }
        }
        ExprKind::Throw(inner) => {
            if let Some(name) = thrown_error_name(inner) {
                out.insert(name);
            }
        }
        ExprKind::Try(inner) => {
            walk_expr(module, inner, fn_defs, callee_sigs, out);
            propagate_call_errors(module, inner, fn_defs, callee_sigs, out);
        }
        ExprKind::Catch { body, arms } => {
            let mut inner = ErrorSet::new();
            walk_expr(module, body, fn_defs, callee_sigs, &mut inner);
            let handled = catch_handled_set(arms);
            if absorbs_all(&handled) {
                // all errors from body absorbed
            } else {
                let remaining = ErrorSet::subtract(&inner, &handled);
                out.extend(&remaining);
            }
            for arm in arms {
                walk_expr(module, &arm.body, fn_defs, callee_sigs, out);
            }
        }
        ExprKind::Call { func, args } => {
            walk_expr(module, func, fn_defs, callee_sigs, out);
            for arg in args {
                walk_expr(module, arg, fn_defs, callee_sigs, out);
            }
            propagate_call_errors(module, func, fn_defs, callee_sigs, out);
        }
        ExprKind::MethodCall { receiver, args, .. } => {
            walk_expr(module, receiver, fn_defs, callee_sigs, out);
            for arg in args {
                walk_expr(module, arg, fn_defs, callee_sigs, out);
            }
        }
        ExprKind::Bind { value, .. } => walk_expr(module, value, fn_defs, callee_sigs, out),
        ExprKind::Assign { value, .. } => walk_expr(module, value, fn_defs, callee_sigs, out),
        ExprKind::Return(Some(v)) => walk_expr(module, v, fn_defs, callee_sigs, out),
        ExprKind::Binary { left, right, .. } => {
            walk_expr(module, left, fn_defs, callee_sigs, out);
            walk_expr(module, right, fn_defs, callee_sigs, out);
        }
        ExprKind::Unary { expr: inner, .. } => walk_expr(module, inner, fn_defs, callee_sigs, out),
        ExprKind::Field { base, .. } => walk_expr(module, base, fn_defs, callee_sigs, out),
        ExprKind::Pipe { left, right } => {
            walk_expr(module, left, fn_defs, callee_sigs, out);
            walk_expr(module, right, fn_defs, callee_sigs, out);
        }
        ExprKind::StructLit { fields, .. } => {
            for f in fields {
                walk_expr(module, &f.value, fn_defs, callee_sigs, out);
            }
        }
        ExprKind::Str(parts) => {
            for part in &parts.0 {
                if let crisp_ast::expr::StringPart::Expr(e) = part {
                    walk_expr(module, e, fn_defs, callee_sigs, out);
                }
            }
        }
        _ => {}
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
}
