//! Milestone 1.0 feature tests: patterns, kitchen_sink, ownership (emit side).
//! LSP overlay / hint coverage lives in `crisp-lsp` (avoids crates.io publish cycle).

use crisp_cir::CirBuilder;
use crisp_rust_emit::{PipelineError, emit_crate, run_emitted, run_tests};
use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn m10_patterns_match_guards_and_tuples() {
    let root = example("patterns");
    eprintln!("patterns cir");
    let cir = CirBuilder::build_crate(&root).expect("cir");
    let out = emit_crate(&cir);
    eprintln!("patterns emit:\n{}", out.lib_rs);
    assert!(out.lib_rs.contains("match "));
    match run_tests(&root) {
        Ok(r) => {
            eprintln!("patterns tests: runtime={}", r.runtime_passed);
            assert!(r.runtime_passed >= 3);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => eprintln!("SKIP patterns tests"),
        Err(e) => panic!("{e}"),
    }
    match run_emitted(&root) {
        Ok(out) => {
            eprintln!("patterns run: {out:?}");
            assert!(out.contains("tagged"));
        }
        Err(PipelineError::ToolchainUnavailable) => eprintln!("SKIP patterns run"),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn m10_kitchen_sink_integrated() {
    let root = example("kitchen_sink");
    TypeChecker::check_crate(&root).expect("typeck kitchen_sink");
    match run_emitted(&root) {
        Ok(out) => assert!(out.contains("port=3000")),
        Err(PipelineError::ToolchainUnavailable) => eprintln!("SKIP kitchen_sink run"),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn m10_ownership_demo_analyze_and_run() {
    let root = example("ownership_demo");
    TypeChecker::check_crate(&root).expect("typeck ownership_demo");
    match run_emitted(&root) {
        Ok(out) => {
            eprintln!("ownership run: {out:?}");
            assert!(out.contains("crisp"));
        }
        Err(PipelineError::ToolchainUnavailable) => eprintln!("SKIP ownership run"),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn m10_fallible_typechecks() {
    let root = example("fallible");
    TypeChecker::check_crate(&root).expect("typeck fallible");
}
