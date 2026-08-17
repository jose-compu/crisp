//! Emit and run `test` / `test_compile_fail` items (spec §19).

use crate::cargo::{CargoError, cargo_test};
use crate::emit::emit_crate;
use crate::pipeline::{PipelineError, analyze_and_build_cir};
use crate::project::{with_emit_dir_lock, write_cargo_project_unlocked};
use crate::seal::verify_sealed_api;
use anyhow::{Context, Result};
use crisp_ast::expr::{Block, Expr, ExprKind, Stmt};
use crisp_ast::item::{Item, TestDef};
use crisp_ast::pat::PatKind;
use crisp_manifest::{read_manifest, resolve_dependencies};
use crisp_resolve::module::load_module_graph;
use crisp_typeck::TypeChecker;
use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TestHarnessError {
    #[error("{0}")]
    Pipeline(#[from] PipelineError),
    #[error("{0}")]
    Cargo(#[from] CargoError),
    #[error("[E0081] runtime test `{name}` failed: {output}")]
    RuntimeFailed { name: String, output: String },
    #[error("[E0082] compile-fail test `{name}` passed but should have failed analysis")]
    CompileFailPassed { name: String },
    #[error("[E0083] compile-fail test `{name}` failed for wrong reason: {reason}")]
    CompileFailWrongReason { name: String, reason: String },
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub struct CollectedTest {
    pub module: String,
    pub name: String,
    pub compile_fail: bool,
    pub body: Block,
}

pub struct TestRunReport {
    pub runtime_passed: usize,
    pub compile_fail_passed: usize,
    pub emitted_tests: String,
}

pub fn collect_tests(crate_root: &Path) -> Result<Vec<CollectedTest>> {
    let graph = load_module_graph(crate_root)?;
    let mut tests = Vec::new();
    for (module_path, node) in &graph.modules {
        for item in &node.ast.items {
            match item {
                Item::Test(t) => tests.push(collected_from(module_path, t, false)),
                Item::TestCompileFail(t) => tests.push(collected_from(module_path, t, true)),
                _ => {}
            }
        }
    }
    Ok(tests)
}

fn collected_from(module: &str, t: &TestDef, compile_fail: bool) -> CollectedTest {
    CollectedTest {
        module: module.to_string(),
        name: t.name.clone(),
        compile_fail,
        body: t.body.clone(),
    }
}

pub fn emit_test_module(tests: &[CollectedTest]) -> String {
    let runtime: Vec<_> = tests.iter().filter(|t| !t.compile_fail).collect();
    if runtime.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "\n#[cfg(test)]\nmod crisp_tests {\n    use super::*;\n    #[allow(unused_imports)]\n    use crate::Show;\n    #[allow(unused_imports)]\n    use crate::Eq;\n    #[allow(unused_imports)]\n    use crate::Ord;\n\n",
    );
    let mut used = HashSet::new();
    for t in runtime {
        let fn_name = unique_test_fn_name(&mut used, &t.module, &t.name);
        let _ = writeln!(out, "    #[test]");
        let _ = writeln!(out, "    fn {fn_name}() {{");
        emit_block(&mut out, &t.body, 2);
        let _ = writeln!(out, "    }}\n");
    }
    out.push_str("}\n");
    out
}

fn unique_test_fn_name(used: &mut HashSet<String>, module: &str, name: &str) -> String {
    let base = sanitize_test_name(module, name);
    if used.insert(base.clone()) {
        return base;
    }
    let mut i = 2u32;
    loop {
        let candidate = format!("{base}_{i}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        i += 1;
    }
}

fn sanitize_ident_part(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if (ch.is_whitespace() || ch == '-' || ch == '_' || ch == '.')
            && !out.ends_with('_')
            && !out.is_empty()
        {
            out.push('_');
        }
    }
    out
}

fn sanitize_test_name(module: &str, name: &str) -> String {
    let mod_s = sanitize_ident_part(module);
    let name_s = sanitize_ident_part(name);
    let combined = if mod_s.is_empty() {
        name_s
    } else if name_s.is_empty() {
        mod_s
    } else {
        format!("{mod_s}_{name_s}")
    };
    // Prefix so generated `fn` names cannot shadow `use super::*` items
    // (e.g. test "proxy demo" vs `pub fn proxy_demo`).
    if combined.is_empty() {
        "test_unnamed".into()
    } else if combined.starts_with("test_") {
        combined
    } else {
        format!("test_{combined}")
    }
}

fn emit_block(out: &mut String, block: &Block, indent: usize) {
    let pad = " ".repeat(indent);
    for stmt in &block.stmts {
        match stmt {
            Stmt::Expr(e) => {
                let _ = writeln!(out, "{pad}{};", emit_expr(e));
            }
            Stmt::Bind { pat, value, .. } => {
                let name = pat_name(pat);
                let rhs = emit_expr(value);
                let _ = writeln!(out, "{pad}let {name} = {rhs};");
            }
            Stmt::Assign { target, value } => {
                let _ = writeln!(
                    out,
                    "{pad}let mut {name} = {rhs};",
                    name = target.name,
                    rhs = emit_expr(value)
                );
            }
        }
    }
    if let Some(tail) = &block.tail {
        let _ = writeln!(out, "{pad}{};", emit_expr(tail));
    }
}

fn pat_name(pat: &crisp_ast::pat::Pat) -> String {
    match &pat.kind {
        PatKind::Ident(id) => id.name.clone(),
        _ => "_".into(),
    }
}

fn emit_expr(expr: &Expr) -> String {
    if let Some(lifted) = crisp_ast::lift_holes(expr) {
        return emit_expr(&lifted);
    }
    match &expr.kind {
        ExprKind::Int(n) => n.to_string(),
        ExprKind::Float(f) => {
            if f.fract() == 0.0 {
                format!("{f}.0_f64")
            } else {
                format!("{f}_f64")
            }
        }
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Str(parts) => {
            let mut s = String::from("\"");
            for p in &parts.0 {
                if let crisp_ast::expr::StringPart::Lit(l) = p {
                    s.push_str(&l.replace('"', "\\\""));
                }
            }
            s.push('"');
            s
        }
        ExprKind::Ident(id) => id.name.clone(),
        ExprKind::Call { func, args } => {
            // Associated fn / enum ctor / instance method: Field under Call.
            if let ExprKind::Field { base, field } = &func.kind {
                let arg_strs: Vec<_> = args.iter().map(emit_expr).collect();
                if let ExprKind::Ident(ty) = &base.kind
                    && ty
                        .name
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_uppercase())
                {
                    return format!("{}::{}({})", ty.name, field.name, arg_strs.join(", "));
                }
                // Instance: recv.method(args) — including chained AssocCall receivers.
                // Eq.equal / Ord.compare take `&Self`.
                let args_fmt = if matches!(field.name.as_str(), "equal" | "compare") {
                    args.iter()
                        .map(|a| format!("&{}", emit_expr(a)))
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    arg_strs.join(", ")
                };
                return format!("{}.{}({})", emit_expr(base), field.name, args_fmt);
            }
            let callee = match &func.kind {
                ExprKind::Ident(id) => id.name.clone(),
                _ => format!("({})", emit_expr(func)),
            };
            if callee == "assert_eq" {
                let arg_strs: Vec<_> = args.iter().map(emit_expr).collect();
                return emit_assert_eq(args, &arg_strs);
            }
            let arg_strs: Vec<_> = args.iter().map(emit_call_arg_for_test).collect();
            format!("{}({})", callee, arg_strs.join(", "))
        }
        ExprKind::Binary { op, left, right } => {
            let op_str = match op {
                crisp_ast::expr::BinaryOp::Add => "+",
                crisp_ast::expr::BinaryOp::Sub => "-",
                crisp_ast::expr::BinaryOp::Mul => "*",
                crisp_ast::expr::BinaryOp::Div => "/",
                crisp_ast::expr::BinaryOp::Pow => ".powf",
                crisp_ast::expr::BinaryOp::Concat => "+",
                crisp_ast::expr::BinaryOp::Eq => "==",
                crisp_ast::expr::BinaryOp::Ne => "!=",
                crisp_ast::expr::BinaryOp::Lt => "<",
                crisp_ast::expr::BinaryOp::Le => "<=",
                crisp_ast::expr::BinaryOp::Gt => ">",
                crisp_ast::expr::BinaryOp::Ge => ">=",
                crisp_ast::expr::BinaryOp::And => "&&",
                crisp_ast::expr::BinaryOp::Or => "||",
                _ => "+",
            };
            if matches!(op, crisp_ast::expr::BinaryOp::Pow) {
                format!(
                    "(({}) as f64).powf(({}) as f64)",
                    emit_expr(left),
                    emit_expr(right)
                )
            } else {
                format!(
                    "{} {} {}",
                    emit_binop_operand(left, *op, false),
                    op_str,
                    emit_binop_operand(right, *op, true)
                )
            }
        }
        ExprKind::Unary { op, expr: inner } => {
            let op_s = match op {
                crisp_ast::expr::UnaryOp::Neg => "-",
                crisp_ast::expr::UnaryOp::Not => "!",
            };
            let inner_s = emit_expr(inner);
            if matches!(inner.kind, ExprKind::Binary { .. }) {
                format!("{op_s}({inner_s})")
            } else {
                format!("{op_s}{inner_s}")
            }
        }
        ExprKind::Block(b) => {
            let mut inner = String::new();
            emit_block(&mut inner, b, 0);
            format!("{{ {inner} }}")
        }
        ExprKind::Field { base, field } => {
            // Unit enum variant: Color.Red
            if let ExprKind::Ident(ty) = &base.kind
                && ty
                    .name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
            {
                format!("{}::{}", ty.name, field.name)
            } else {
                format!("{}.{}", emit_expr(base), field.name)
            }
        }
        ExprKind::StructLit { name, fields } => {
            if fields.is_empty() {
                format!("{} {{}}", name.name)
            } else {
                let parts: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        let val = emit_expr(&f.value);
                        let val = if matches!(f.value.kind, ExprKind::Str(_)) {
                            format!("{val}.to_string()")
                        } else {
                            val
                        };
                        format!("{}: {val}", f.name.name)
                    })
                    .collect();
                format!("{} {{ {} }}", name.name, parts.join(", "))
            }
        }
        ExprKind::Lambda { params, body } => {
            let names: Vec<_> = params.iter().map(|p| p.name.name.as_str()).collect();
            format!("move |{}| {}", names.join(", "), emit_expr(body))
        }
        _ => "()".into(),
    }
}

fn emit_binop_operand(expr: &Expr, parent: crisp_ast::expr::BinaryOp, is_right: bool) -> String {
    let inner = emit_expr(expr);
    if ast_binop_needs_parens(expr, parent, is_right) {
        format!("({inner})")
    } else {
        inner
    }
}

fn ast_binop_needs_parens(expr: &Expr, parent: crisp_ast::expr::BinaryOp, is_right: bool) -> bool {
    let ExprKind::Binary { op, .. } = &expr.kind else {
        return false;
    };
    if matches!(
        op,
        crisp_ast::expr::BinaryOp::Pow | crisp_ast::expr::BinaryOp::Concat
    ) {
        return false;
    }
    let cp = op.rust_prec();
    let pp = parent.rust_prec();
    cp < pp || (cp == pp && is_right)
}

fn emit_assert_eq(args: &[Expr], emitted: &[String]) -> String {
    // Epsilon only for numeric floats. A nested float must not force `.abs()` on
    // bool / str / comparison (`assert_eq(ignites(1.2, 0.8), true)`, #102).
    if emitted.len() >= 2
        && args.iter().any(contains_float_literal)
        && !args.iter().any(is_non_numeric_eq_side)
    {
        format!("assert!(({} - ({})).abs() < 1e-9)", emitted[0], emitted[1])
    } else {
        format!("assert_eq!({})", emitted.join(", "))
    }
}

fn is_non_numeric_eq_side(expr: &Expr) -> bool {
    matches!(
        &expr.kind,
        ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Char(_)
            | ExprKind::Binary {
                op: crisp_ast::expr::BinaryOp::Eq
                    | crisp_ast::expr::BinaryOp::Ne
                    | crisp_ast::expr::BinaryOp::Lt
                    | crisp_ast::expr::BinaryOp::Le
                    | crisp_ast::expr::BinaryOp::Gt
                    | crisp_ast::expr::BinaryOp::Ge
                    | crisp_ast::expr::BinaryOp::And
                    | crisp_ast::expr::BinaryOp::Or,
                ..
            }
    )
}

fn contains_float_literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Float(_) => true,
        ExprKind::Call { args, .. } => args.iter().any(contains_float_literal),
        ExprKind::Unary { expr: inner, .. } => contains_float_literal(inner),
        ExprKind::Binary { left, right, .. } => {
            contains_float_literal(left) || contains_float_literal(right)
        }
        ExprKind::Field { base, .. } => contains_float_literal(base),
        ExprKind::Block(b) => {
            b.tail.as_ref().is_some_and(|t| contains_float_literal(t))
                || b.stmts.iter().any(|s| match s {
                    Stmt::Expr(e) | Stmt::Bind { value: e, .. } | Stmt::Assign { value: e, .. } => {
                        contains_float_literal(e)
                    }
                })
        }
        _ => false,
    }
}

fn emit_call_arg_for_test(expr: &Expr) -> String {
    match &expr.kind {
        // Copy scalars emit by value in CIR/Rust (Owned); do not borrow literals.
        ExprKind::Int(n) => n.to_string(),
        ExprKind::Float(f) => {
            if f.fract() == 0.0 {
                format!("{f}.0_f64")
            } else {
                format!("{f}_f64")
            }
        }
        ExprKind::Bool(_) | ExprKind::Char(_) => emit_expr(expr),
        // Prefer `&ident` for stringish/`&T` params; copy locals still coerce via Copy.
        ExprKind::Ident(id) => format!("&{}", id.name),
        // Enum values are owned; pass by reference for `&Color` params.
        ExprKind::Field { base, .. } if matches!(&base.kind, ExprKind::Ident(ty) if ty.name.chars().next().is_some_and(|c| c.is_ascii_uppercase())) =>
        {
            format!("&{}", emit_expr(expr))
        }
        ExprKind::Call { func, .. }
            if matches!(
                &func.kind,
                ExprKind::Field { base, .. }
                    if matches!(&base.kind, ExprKind::Ident(ty) if ty.name.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
            ) =>
        {
            format!("&{}", emit_expr(expr))
        }
        // Nested calls that return user types (`parse_fuel("ch4")`) need `&` like CIR (#102).
        ExprKind::Call { func, .. } if matches!(&func.kind, ExprKind::Ident(_)) => {
            format!("&{}", emit_expr(expr))
        }
        _ => emit_expr(expr),
    }
}

pub fn run_tests(crate_root: &Path) -> Result<TestRunReport, TestHarnessError> {
    verify_sealed_api(crate_root).map_err(|e| TestHarnessError::Other(e.into()))?;

    let tests = collect_tests(crate_root)?;
    let compile_fail: Vec<_> = tests.iter().filter(|t| t.compile_fail).collect();
    for t in &compile_fail {
        run_compile_fail_test(t)?;
    }

    let emitted_tests = emit_test_module(&tests);
    let runtime_count = tests.iter().filter(|t| !t.compile_fail).count();

    if runtime_count == 0 {
        return Ok(TestRunReport {
            runtime_passed: 0,
            compile_fail_passed: compile_fail.len(),
            emitted_tests,
        });
    }

    with_emit_dir_lock(crate_root, || {
        let cir = analyze_and_build_cir(crate_root)?;
        let manifest = read_manifest(crate_root).context("read crisp.toml")?;
        let deps = resolve_dependencies(&manifest);
        let emitted = emit_crate(&cir);
        write_cargo_project_unlocked(crate_root, &emitted, &manifest, &deps, Some(&emitted_tests))
            .context("write target/rust with tests")?;

        match cargo_test(crate_root) {
            Err(CargoError::NotFound) => Err(TestHarnessError::Other(anyhow::anyhow!(
                "cargo not on PATH"
            ))),
            Err(CargoError::BuildFailed(output)) => Err(TestHarnessError::RuntimeFailed {
                name: "cargo test".into(),
                output,
            }),
            Err(e) => Err(TestHarnessError::Cargo(e)),
            Ok(()) => Ok(TestRunReport {
                runtime_passed: runtime_count,
                compile_fail_passed: compile_fail.len(),
                emitted_tests,
            }),
        }
    })
}

fn run_compile_fail_test(test: &CollectedTest) -> Result<(), TestHarnessError> {
    let temp = tempfile::TempDir::new().map_err(|e| TestHarnessError::Other(e.into()))?;
    write_probe_crate(temp.path(), test)?;

    match TypeChecker::check_crate(temp.path()) {
        Ok(_) => Err(TestHarnessError::CompileFailPassed {
            name: test.name.clone(),
        }),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("E00") || msg.contains("unknown") || msg.contains("unification") {
                Ok(())
            } else {
                Err(TestHarnessError::CompileFailWrongReason {
                    name: test.name.clone(),
                    reason: msg,
                })
            }
        }
    }
}

fn write_probe_crate(dir: &Path, test: &CollectedTest) -> Result<(), TestHarnessError> {
    std::fs::write(
        dir.join("crisp.toml"),
        r#"[package]
name = "probe"
version = "0.1.0"
edition = "2026"
"#,
    )
    .map_err(|e| TestHarnessError::Other(e.into()))?;
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).map_err(|e| TestHarnessError::Other(e.into()))?;
    let body = block_to_crisp(&test.body);
    let main = format!("probe() = {{\n{body}\n}}\n\npub main() = probe()\n");
    std::fs::write(src_dir.join("main.crp"), main)
        .map_err(|e| TestHarnessError::Other(e.into()))?;
    Ok(())
}

fn block_to_crisp(block: &Block) -> String {
    let mut lines = Vec::new();
    for stmt in &block.stmts {
        match stmt {
            Stmt::Expr(e) => lines.push(emit_crisp_expr(e)),
            Stmt::Bind { pat, value, .. } => {
                lines.push(format!("{} := {}", pat_name(pat), emit_crisp_expr(value)));
            }
            Stmt::Assign { target, value } => {
                lines.push(format!("{} := {}", target.name, emit_crisp_expr(value)));
            }
        }
    }
    if let Some(tail) = &block.tail {
        lines.push(emit_crisp_expr(tail));
    }
    lines
        .into_iter()
        .map(|l| format!("    {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn emit_crisp_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Int(n) => n.to_string(),
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Str(parts) => {
            let mut s = String::from("\"");
            for p in &parts.0 {
                if let crisp_ast::expr::StringPart::Lit(l) = p {
                    s.push_str(l);
                }
            }
            s.push('"');
            s
        }
        ExprKind::Ident(id) => id.name.clone(),
        ExprKind::Call { func, args } => {
            let name = match &func.kind {
                ExprKind::Ident(id) => id.name.clone(),
                _ => "unknown".into(),
            };
            let arg_strs: Vec<_> = args.iter().map(emit_crisp_expr).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        ExprKind::Binary { op, left, right } => match op {
            crisp_ast::expr::BinaryOp::Concat => {
                format!("{} ++ {}", emit_crisp_expr(left), emit_crisp_expr(right))
            }
            _ => format!("{} + {}", emit_crisp_expr(left), emit_crisp_expr(right)),
        },
        _ => "()".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_names() {
        assert_eq!(
            sanitize_test_name("main", "greet works"),
            "test_main_greet_works"
        );
        assert_eq!(sanitize_test_name("main", "A-B"), "test_main_a_b");
        assert_eq!(
            sanitize_test_name("main", "proxy demo"),
            "test_main_proxy_demo"
        );
        assert_eq!(
            sanitize_test_name("analysis.ignition", "wide kernel ignites"),
            "test_analysis_ignition_wide_kernel_ignites"
        );
        assert_eq!(
            sanitize_test_name("main", "test_already"),
            "test_main_test_already"
        );
    }

    #[test]
    fn emit_assert_eq_test() {
        let tests = vec![CollectedTest {
            module: "main".into(),
            name: "addition".into(),
            compile_fail: false,
            body: Block {
                stmts: vec![Stmt::Expr(Expr {
                    kind: ExprKind::Call {
                        func: Box::new(Expr {
                            kind: ExprKind::Ident(crisp_ast::ident::Ident {
                                name: "assert_eq".into(),
                                span: Default::default(),
                            }),
                            span: Default::default(),
                        }),
                        args: vec![
                            Expr {
                                kind: ExprKind::Int(2),
                                span: Default::default(),
                            },
                            Expr {
                                kind: ExprKind::Int(2),
                                span: Default::default(),
                            },
                        ],
                    },
                    span: Default::default(),
                })],
                tail: None,
                span: Default::default(),
            },
        }];
        let out = emit_test_module(&tests);
        assert!(out.contains("assert_eq!"));
        assert!(out.contains("fn test_main_addition"));
    }

    #[test]
    fn emit_float_assert_uses_epsilon() {
        let tests = vec![CollectedTest {
            module: "main".into(),
            name: "float add".into(),
            compile_fail: false,
            body: Block {
                stmts: vec![Stmt::Expr(Expr {
                    kind: ExprKind::Call {
                        func: Box::new(Expr {
                            kind: ExprKind::Ident(crisp_ast::ident::Ident {
                                name: "assert_eq".into(),
                                span: Default::default(),
                            }),
                            span: Default::default(),
                        }),
                        args: vec![
                            Expr {
                                kind: ExprKind::Float(1.5),
                                span: Default::default(),
                            },
                            Expr {
                                kind: ExprKind::Float(1.5),
                                span: Default::default(),
                            },
                        ],
                    },
                    span: Default::default(),
                })],
                tail: None,
                span: Default::default(),
            },
        }];
        let out = emit_test_module(&tests);
        assert!(out.contains(".abs() < 1e-9"));
        assert!(out.contains("fn test_main_float_add"));
    }

    fn ident_expr(name: &str) -> Expr {
        Expr {
            kind: ExprKind::Ident(crisp_ast::ident::Ident {
                name: name.into(),
                span: Default::default(),
            }),
            span: Default::default(),
        }
    }

    fn call_assert_eq(args: Vec<Expr>) -> Stmt {
        Stmt::Expr(Expr {
            kind: ExprKind::Call {
                func: Box::new(ident_expr("assert_eq")),
                args,
            },
            span: Default::default(),
        })
    }

    #[test]
    fn duplicate_titles_get_module_prefixed_names() {
        let tests = vec![
            CollectedTest {
                module: "analysis.ignition".into(),
                name: "wide kernel ignites".into(),
                compile_fail: false,
                body: Block {
                    stmts: vec![],
                    tail: None,
                    span: Default::default(),
                },
            },
            CollectedTest {
                module: "failure.relight".into(),
                name: "wide kernel ignites".into(),
                compile_fail: false,
                body: Block {
                    stmts: vec![],
                    tail: None,
                    span: Default::default(),
                },
            },
        ];
        let out = emit_test_module(&tests);
        assert!(out.contains("fn test_analysis_ignition_wide_kernel_ignites"));
        assert!(out.contains("fn test_failure_relight_wide_kernel_ignites"));
    }

    #[test]
    fn assert_eq_bool_with_nested_float_is_not_epsilon() {
        let tests = vec![CollectedTest {
            module: "main".into(),
            name: "bool".into(),
            compile_fail: false,
            body: Block {
                stmts: vec![call_assert_eq(vec![
                    Expr {
                        kind: ExprKind::Call {
                            func: Box::new(ident_expr("ignites")),
                            args: vec![
                                Expr {
                                    kind: ExprKind::Float(1.2),
                                    span: Default::default(),
                                },
                                Expr {
                                    kind: ExprKind::Float(0.8),
                                    span: Default::default(),
                                },
                            ],
                        },
                        span: Default::default(),
                    },
                    Expr {
                        kind: ExprKind::Bool(true),
                        span: Default::default(),
                    },
                ])],
                tail: None,
                span: Default::default(),
            },
        }];
        let out = emit_test_module(&tests);
        assert!(out.contains("assert_eq!"), "{out}");
        assert!(!out.contains(".abs()"), "{out}");
    }

    #[test]
    fn assert_eq_str_is_not_epsilon() {
        let tests = vec![CollectedTest {
            module: "main".into(),
            name: "str".into(),
            compile_fail: false,
            body: Block {
                stmts: vec![call_assert_eq(vec![
                    ident_expr("s"),
                    Expr {
                        kind: ExprKind::Str(crisp_ast::expr::StringParts(vec![
                            crisp_ast::expr::StringPart::Lit("FLASHBACK".into()),
                        ])),
                        span: Default::default(),
                    },
                ])],
                tail: None,
                span: Default::default(),
            },
        }];
        let out = emit_test_module(&tests);
        assert!(out.contains("assert_eq!"), "{out}");
        assert!(!out.contains(".abs()"), "{out}");
    }

    fn float_lit(v: f64) -> Expr {
        Expr {
            kind: ExprKind::Float(v),
            span: Default::default(),
        }
    }

    #[test]
    fn assert_eq_float_rhs_binop_is_parenthesized() {
        let rhs = Expr {
            kind: ExprKind::Binary {
                op: crisp_ast::expr::BinaryOp::Sub,
                left: Box::new(float_lit(0.0)),
                right: Box::new(float_lit(2.0)),
            },
            span: Default::default(),
        };
        let tests = vec![CollectedTest {
            module: "main".into(),
            name: "paren rhs".into(),
            compile_fail: false,
            body: Block {
                stmts: vec![call_assert_eq(vec![ident_expr("a"), rhs])],
                tail: None,
                span: Default::default(),
            },
        }];
        let out = emit_test_module(&tests);
        assert!(
            out.contains("(a - (0.0_f64 - 2.0_f64)).abs()"),
            "RHS must be parenthesized (#113):\n{out}"
        );
    }

    #[test]
    fn assert_eq_float_unary_minus_uses_epsilon() {
        let rhs = Expr {
            kind: ExprKind::Unary {
                op: crisp_ast::expr::UnaryOp::Neg,
                expr: Box::new(float_lit(2.0)),
            },
            span: Default::default(),
        };
        let tests = vec![CollectedTest {
            module: "main".into(),
            name: "unary".into(),
            compile_fail: false,
            body: Block {
                stmts: vec![call_assert_eq(vec![ident_expr("a"), rhs])],
                tail: None,
                span: Default::default(),
            },
        }];
        let out = emit_test_module(&tests);
        assert!(out.contains(".abs() < 1e-9"), "{out}");
        assert!(out.contains("-2.0_f64"), "{out}");
    }
}
