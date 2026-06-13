//! Complex feature matrix — examples + runtime tests for all advanced Crisp features.

use crisp_cir::CirBuilder;
use crisp_lsp::CrispAnalysis;
use crisp_rust_emit::{
    build_emitted, collect_tests, emit_crate, run_emitted, run_tests, verify_sealed_api,
    PipelineError,
};
use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(name)
}

const MATRIX: &[(&str, &[&str])] = &[
    ("match", &["patterns", "workshop", "match"]),
    ("async_spawn", &["async_hello", "async_spawn"]),
    ("ffi_unsafe", &["ffi", "unsafe_math"]),
    ("fallible", &["fallible", "fallible_chain", "inventory", "kitchen_sink"]),
    ("stdlib_vec", &["stdlib_smoke", "vec_ops", "data_pipeline"]),
    ("multi_module", &["server", "math", "workshop", "inventory"]),
    ("defaults", &["defaults", "server", "kitchen_sink"]),
    ("sealed", &["sealed"]),
    ("ownership", &["ownership_demo"]),
    ("tests_harness", &["with_tests", "math", "workshop", "vec_ops", "unsafe_math"]),
];

#[test]
fn complex_matrix_typechecks() {
    for (feature, examples) in MATRIX {
        eprintln!("feature={feature}");
        for name in *examples {
            eprintln!("  typecheck {name}");
            TypeChecker::check_crate(&example(name))
                .unwrap_or_else(|e| panic!("{name} ({feature}): {e}"));
        }
    }
}

#[test]
fn complex_vec_ops_push_emit() {
    let cir = CirBuilder::build_crate(&example("vec_ops")).expect("cir");
    let out = emit_crate(&cir);
    eprintln!("vec_ops emit:\n{}", out.lib_rs);
    assert!(out.lib_rs.contains("Vec::<i64>::new()"));
    assert!(out.lib_rs.contains(".push("));
    assert!(out.lib_rs.contains("as i64"));
}

#[test]
fn complex_fallible_chain_emits_crisp_error() {
    let cir = CirBuilder::build_crate(&example("fallible_chain")).expect("cir");
    assert!(cir.crisp_error.variants.len() >= 3);
    let out = emit_crate(&cir);
    assert!(out.lib_rs.contains("CrispError"));
    assert!(out.lib_rs.contains("fetch()?"));
}

#[test]
fn complex_async_spawn_emits_tokio_spawn() {
    let cir = CirBuilder::build_crate(&example("async_spawn")).expect("cir");
    let out = emit_crate(&cir);
    eprintln!("async_spawn:\n{}", out.lib_rs);
    assert!(out.lib_rs.contains("tokio::spawn"));
    assert!(out.lib_rs.contains("#[tokio::main]"));
}

#[test]
fn complex_workshop_multimodule_tests() {
    let root = example("workshop");
    let tests = collect_tests(&root).expect("collect");
    eprintln!("workshop tests: {tests:?}");
    assert!(tests.iter().any(|t| t.name.contains("greet")));
    match run_tests(&root) {
        Ok(r) => {
            eprintln!("workshop runtime={} compile_fail={}", r.runtime_passed, r.compile_fail_passed);
            assert!(r.runtime_passed >= 3);
            assert!(r.compile_fail_passed >= 1);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => eprintln!("SKIP workshop tests"),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn complex_inventory_lsp_and_fallible() {
    let root = example("inventory");
    let a = CrispAnalysis::analyze(&root).expect("analyze");
    let overlays = a.call_overlays(&root.join("src/main.crp")).unwrap();
    assert!(overlays.iter().any(|o| o.callee == "try_reserve" || o.callee == "lookup_item"));
}

#[test]
fn complex_sealed_lock_still_valid() {
    verify_sealed_api(&example("sealed")).expect("sealed lock");
}

#[test]
fn complex_runtime_smoke_all_new_examples() {
    for name in [
        "vec_ops",
        "fallible_chain",
        "async_spawn",
        "workshop",
        "unsafe_math",
        "data_pipeline",
    ] {
        eprintln!("run {name}");
        match run_emitted(&example(name)) {
            Ok(out) => {
                eprintln!("  {out:?}");
                assert!(!out.is_empty(), "{name} empty output");
            }
            Err(PipelineError::ToolchainUnavailable) => eprintln!("  SKIP (no cargo)"),
            Err(e) => panic!("{name}: {e}"),
        }
    }
}

#[test]
fn complex_all_new_examples_build() {
    for name in [
        "vec_ops",
        "fallible_chain",
        "async_spawn",
        "workshop",
        "unsafe_math",
        "data_pipeline",
    ] {
        eprintln!("build {name}");
        match build_emitted(&example(name)) {
            Ok(_) => eprintln!("  ok"),
            Err(PipelineError::ToolchainUnavailable) => eprintln!("  SKIP"),
            Err(e) => panic!("{name}: {e}"),
        }
    }
}

#[test]
fn complex_unsafe_math_emits_nested_unsafe() {
    let out = emit_crate(&CirBuilder::build_crate(&example("unsafe_math")).unwrap());
    assert!(out.lib_rs.contains("unsafe"));
    assert!(out.lib_rs.contains("extern \"C\""));
}
