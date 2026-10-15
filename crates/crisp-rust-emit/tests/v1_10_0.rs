//! v1.10.0: slices (#159), vec<str>/vec<bool> FFI (#158), trait/dyn diagnostics (#59).

use crisp_cir::CirBuilder;
use crisp_rust_emit::{emit_crate, run_tests};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn issue_159_emit_slice_to_vec() {
    let cir = CirBuilder::build_crate(&example("vec_ops")).expect("cir #159");
    let out = emit_crate(&cir);
    eprintln!("#159 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains(".to_vec()"),
        "slice should emit .to_vec():\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("as usize..(") || out.lib_rs.contains("as usize.."),
        "slice range emit:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_159_runtime_slice() {
    match run_tests(&example("vec_ops")) {
        Ok(r) => {
            eprintln!(
                "#159 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert!(
                r.runtime_passed >= 5,
                "expected vec_ops tests including slice, got {}",
                r.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_159 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_159 harness: {e}"),
    }
}

#[test]
fn issue_158_emit_vec_str_bool_as_slice() {
    let cir = CirBuilder::build_crate(&example("path_dep")).expect("cir #158");
    let out = emit_crate(&cir);
    eprintln!("#158 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("local_core::join_names"),
        "missing join_names:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("local_core::all_true"),
        "missing all_true:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains(".as_slice()"),
        "vec FFI should emit .as_slice():\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_158_runtime_vec_str_bool() {
    match run_tests(&example("path_dep")) {
        Ok(r) => {
            eprintln!(
                "#158 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert!(
                r.runtime_passed >= 6,
                "expected path_dep vec str/bool tests, got {}",
                r.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_158 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_158 harness: {e}"),
    }
}

#[test]
fn issue_59_trait_defaults_still_pass() {
    match run_tests(&example("trait_defaults")) {
        Ok(r) => {
            eprintln!("#59 runtime={}", r.runtime_passed);
            assert!(
                r.runtime_passed >= 1,
                "trait_defaults should still pass, got {}",
                r.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_59 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_59 harness: {e}"),
    }
}
