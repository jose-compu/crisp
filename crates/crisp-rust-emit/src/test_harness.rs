//! Emit and run `test` / `test_compile_fail` items (spec §19).

use crate::cargo::{CargoError, cargo_test};
use crate::emit::emit_crate;
use crate::pipeline::{PipelineError, analyze_and_build_cir};
use crate::project::write_cargo_project;
use crate::seal::verify_sealed_api;
use crisp_manifest::{read_manifest, resolve_dependencies};
use anyhow::{Context, Result};
use crisp_ast::expr::{Block, Expr, ExprKind, Stmt};
use crisp_ast::item::{Item, TestDef};
use crisp_ast::pat::PatKind;
use crisp_resolve::module::load_module_graph;
use crisp_typeck::TypeChecker;
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
    let mut out = String::from("\n#[cfg(test)]\nmod crisp_tests {\n    use super::*;\n\n");
    for t in runtime {
        let fn_name = sanitize_test_name(&t.name);
        let _ = writeln!(out, "    #[test]");
        let _ = writeln!(out, "    fn {fn_name}() {{");
        emit_block(&mut out, &t.body, 2);
        let _ = writeln!(out, "    }}\n");
    }
    out.push_str("}\n");
    out
}

fn sanitize_test_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !out.ends_with('_') && !out.is_empty() {
                out.push('_');
            }
        }
    }
    if out.is_empty() {
        "unnamed_test".into()
    } else {
        out
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
    match &expr.kind {
        ExprKind::Int(n) => n.to_string(),
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
            let name = match &func.kind {
                ExprKind::Ident(id) => id.name.clone(),
                _ => "unknown".into(),
            };
            let arg_strs: Vec<_> = args.iter().map(emit_expr).collect();
            if name == "assert_eq" {
                format!("assert_eq!({})", arg_strs.join(", "))
            } else {
                format!("{}({})", name, arg_strs.join(", "))
            }
        }
        ExprKind::Binary { op, left, right } => {
            let op_str = match op {
                crisp_ast::expr::BinaryOp::Add => "+",
                crisp_ast::expr::BinaryOp::Concat => "+",
                _ => "+",
            };
            format!("{} {} {}", emit_expr(left), op_str, emit_expr(right))
        }
        ExprKind::Block(b) => {
            let mut inner = String::new();
            emit_block(&mut inner, b, 0);
            format!("{{ {inner} }}")
        }
        _ => "()".into(),
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

    let cir = analyze_and_build_cir(crate_root)?;
    let manifest = read_manifest(crate_root).context("read crisp.toml")?;
    let deps = resolve_dependencies(&manifest);
    let emitted = emit_crate(&cir);
    write_cargo_project(
        crate_root,
        &emitted,
        &manifest,
        &deps,
        Some(&emitted_tests),
    )
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
    let main = format!(
        "probe() = {{\n{body}\n}}\n\npub main() = probe()\n"
    );
    std::fs::write(src_dir.join("main.crp"), main).map_err(|e| TestHarnessError::Other(e.into()))?;
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
        assert_eq!(sanitize_test_name("greet works"), "greet_works");
        assert_eq!(sanitize_test_name("A-B"), "a_b");
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
        assert!(out.contains("fn addition"));
    }
}
