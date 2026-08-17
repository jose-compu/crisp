//! v1.8.0 regressions.

use crisp_cir::{CirBuilder, CirExpr, CirItem, CirUnaryOp};
use crisp_rust_emit::{
    collect_tests, emit_crate, emit_test_module, emit_test_module_with_cir, run_tests,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn has_unary_neg(expr: &CirExpr) -> bool {
    match expr {
        CirExpr::Unary {
            op: CirUnaryOp::Neg,
            ..
        } => true,
        CirExpr::Unary { expr: inner, .. }
        | CirExpr::Clone { expr: inner, .. }
        | CirExpr::Borrow { expr: inner, .. }
        | CirExpr::Throw { payload: inner, .. }
        | CirExpr::Try { expr: inner, .. }
        | CirExpr::Field { base: inner, .. }
        | CirExpr::Print { arg: inner, .. }
        | CirExpr::Await { expr: inner, .. } => has_unary_neg(inner),
        CirExpr::BinOp { left, right, .. } => has_unary_neg(left) || has_unary_neg(right),
        CirExpr::Call { args, .. } => args.iter().any(|a| has_unary_neg(&a.expr)),
        CirExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            has_unary_neg(cond)
                || has_unary_neg(then_branch)
                || else_branch.as_ref().is_some_and(|e| has_unary_neg(e))
        }
        CirExpr::Block(b) => b.tail.as_ref().is_some_and(|t| has_unary_neg(t)),
        _ => false,
    }
}

#[test]
fn issue_113_cir_lowers_unary_neg() {
    let cir = CirBuilder::build_crate(&fixture("issue_113_unary")).expect("cir #113");
    let main = cir.modules.iter().find(|m| m.path == "main").expect("main");
    let negf = main
        .items
        .iter()
        .find_map(|i| match i {
            CirItem::Function(f) if f.name == "negf" => Some(f),
            _ => None,
        })
        .expect("negf");
    let tail = negf.body.tail.as_deref().expect("negf tail");
    assert!(
        has_unary_neg(tail),
        "negf body should be CirExpr::Unary Neg, got {tail:?}"
    );
}

#[test]
fn issue_113_emit_unary_and_harness_parens() {
    let cir = CirBuilder::build_crate(&fixture("issue_113_unary")).expect("cir #113");
    let out = emit_crate(&cir);
    eprintln!("#113 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("-x") || out.lib_rs.contains("- x"),
        "unary minus should emit, got:\n{}",
        out.lib_rs
    );

    let tests = collect_tests(&fixture("issue_113_unary")).expect("collect #113");
    let emitted = emit_test_module(&tests);
    eprintln!("#113 tests:\n{emitted}");
    assert!(
        emitted.contains("-2.0_f64"),
        "unary minus literal in harness:\n{emitted}"
    );
    assert!(
        emitted.contains("0.0_f64 - 2.0_f64") && emitted.contains(" - ("),
        "binop RHS must be parenthesized:\n{emitted}"
    );
}

#[test]
fn issue_113_runtime_tests() {
    match run_tests(&fixture("issue_113_unary")) {
        Ok(r) => {
            eprintln!(
                "#113 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert_eq!(r.runtime_passed, 2, "got {}", r.runtime_passed);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_113 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_113 harness: {e}"),
    }
}

#[test]
fn issue_112_emit_widening_and_as_float() {
    let cir = CirBuilder::build_crate(&fixture("issue_112_widen")).expect("cir #112");
    let out = emit_crate(&cir);
    eprintln!("#112 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("as f64"),
        "int→float should emit `as f64`:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("1.0_f64"),
        "int literal in a float slot should emit a float literal:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_112_runtime_tests() {
    match run_tests(&fixture("issue_112_widen")) {
        Ok(r) => {
            eprintln!(
                "#112 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert_eq!(r.runtime_passed, 2, "got {}", r.runtime_passed);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_112 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_112 harness: {e}"),
    }
}

#[test]
fn issue_118_copy_records_and_clone_at_bind() {
    let cir = CirBuilder::build_crate(&fixture("issue_118_copy")).expect("cir #118");
    let out = emit_crate(&cir);
    eprintln!("#118 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("#[derive(Debug, Clone, Copy)]") && out.lib_rs.contains("struct YT"),
        "all-Copy record should derive Copy:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("struct Label")
            && out.lib_rs.contains("#[derive(Debug, Clone)]")
            && !out.lib_rs.contains("Copy)]\nstruct Label"),
        "string record must not be Copy:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains(".clone()"),
        "reuse after bind should clone-at-bind:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_118_runtime_tests() {
    match run_tests(&fixture("issue_118_copy")) {
        Ok(r) => {
            eprintln!(
                "#118 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert_eq!(r.runtime_passed, 1, "got {}", r.runtime_passed);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_118 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_118 harness: {e}"),
    }
}

#[test]
fn issue_114_harness_matches_cir_ownership() {
    let root = fixture("issue_114_harness");
    let cir = CirBuilder::build_crate(&root).expect("cir #114");
    let tests = collect_tests(&root).expect("collect #114");
    let emitted = emit_test_module_with_cir(&tests, Some(&cir));
    eprintln!("#114 tests:\n{emitted}");
    assert!(
        emitted.contains("hold_0d(unburnt_yt()") && !emitted.contains("hold_0d(&unburnt_yt()"),
        "owned record call must not grow an extra `&`:\n{emitted}"
    );
    assert!(
        emitted.contains("rhs_point(")
            && emitted.contains("lap3(")
            && !emitted.contains("&lap3(")
            && !emitted.contains("&upwind("),
        "owned float nested calls must not grow `&`:\n{emitted}"
    );
    assert!(
        emitted.contains("step(u)") && !emitted.contains("step(&u)"),
        "owned ident must not grow `&`:\n{emitted}"
    );
    assert!(
        emitted.contains("describe(&fuel_ch4()") || emitted.contains("describe(fuel_ch4("),
        "nested record arg should emit:\n{emitted}"
    );
}

#[test]
fn issue_114_runtime_tests() {
    match run_tests(&fixture("issue_114_harness")) {
        Ok(r) => {
            eprintln!(
                "#114 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert_eq!(r.runtime_passed, 4, "got {}", r.runtime_passed);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_114 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_114 harness: {e}"),
    }
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn issue_119_emit_vec_t() {
    let cir = CirBuilder::build_crate(&example("vec_ops")).expect("cir #119");
    let out = emit_crate(&cir);
    eprintln!("#119 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("Vec::<i64>::new()") || out.lib_rs.contains("Vec::<i64>"),
        "int vec emit:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("vec![") && out.lib_rs.contains("1.0_f64"),
        "float vec literal emit:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_119_runtime_tests() {
    match run_tests(&example("vec_ops")) {
        Ok(r) => {
            eprintln!(
                "#119 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert_eq!(r.runtime_passed, 3, "got {}", r.runtime_passed);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_119 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_119 harness: {e}"),
    }
}
