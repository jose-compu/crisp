//! v1.11.0: JSON object/array emit (#166/#169), fixed `[T; N]` (#167).

use crisp_cir::CirBuilder;
use crisp_rust_emit::{emit_crate, run_tests};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn issue_166_emit_json_object() {
    let cir = CirBuilder::build_crate(&example("json_literals")).expect("cir #166");
    let out = emit_crate(&cir);
    eprintln!("#166 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("serde_json::json!({"),
        "object should emit json!:\n{}",
        out.lib_rs
    );
    assert!(out.lib_rs.contains("null"), "none -> null:\n{}", out.lib_rs);
}

#[test]
fn issue_169_emit_json_array() {
    let cir = CirBuilder::build_crate(&example("json_literals")).expect("cir #169");
    let out = emit_crate(&cir);
    assert!(
        out.lib_rs.contains("serde_json::json!(["),
        "array should emit json!:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_166_runtime_json_literals() {
    match run_tests(&example("json_literals")) {
        Ok(r) => {
            eprintln!(
                "#166 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert!(
                r.runtime_passed >= 3,
                "expected json_literals tests, got {}",
                r.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_166 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_166 harness: {e}"),
    }
}

#[test]
fn issue_167_emit_fixed_array() {
    let cir = CirBuilder::build_crate(&example("fixed_arrays")).expect("cir #167");
    let out = emit_crate(&cir);
    eprintln!("#167 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("[f64; 3]"),
        "fixed array type:\n{}",
        out.lib_rs
    );
    assert!(
        !out.lib_rs.contains("fn triple() -> Vec"),
        "triple must not be Vec:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_167_runtime_fixed_arrays() {
    match run_tests(&example("fixed_arrays")) {
        Ok(r) => {
            eprintln!(
                "#167 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert!(
                r.runtime_passed >= 3,
                "expected fixed_arrays tests, got {}",
                r.runtime_passed
            );
            assert!(
                r.compile_fail_passed >= 1,
                "expected wrong-length compile_fail"
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_167 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_167 harness: {e}"),
    }
}
