//! End-to-end tests across all `examples/` crates.

use crisp_cir::CirBuilder;
use crisp_manifest::read_manifest;
use crisp_resolve::Resolver;
use crisp_rust_emit::{
    build_emitted, collect_tests, emit_to_target, run_emitted, run_tests, verify_sealed_api,
    PipelineError,
};
use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn example(name: &str) -> PathBuf {
    examples_dir().join(name)
}

const EXAMPLES: &[&str] = &[
    "hello",
    "server",
    "fallible",
    "with_tests",
    "math",
    "defaults",
    "sealed",
];

#[test]
fn all_examples_resolve() {
    for name in EXAMPLES {
        let root = example(name);
        eprintln!("resolve: {name}");
        Resolver::resolve_crate(&root).unwrap_or_else(|e| panic!("{name} resolve: {e}"));
    }
}

#[test]
fn all_examples_typecheck() {
    for name in EXAMPLES {
        let root = example(name);
        eprintln!("typecheck: {name}");
        TypeChecker::check_crate(&root).unwrap_or_else(|e| panic!("{name} typecheck: {e}"));
    }
}

#[test]
fn all_examples_build_cir() {
    for name in EXAMPLES {
        let root = example(name);
        eprintln!("cir: {name}");
        let cir = CirBuilder::build_crate(&root).unwrap_or_else(|e| panic!("{name} cir: {e}"));
        assert!(!cir.modules.is_empty(), "{name} has no modules");
    }
}

#[test]
fn all_examples_emit() {
    for name in EXAMPLES {
        let root = example(name);
        eprintln!("emit: {name}");
        let out = emit_to_target(&root).unwrap_or_else(|e| panic!("{name} emit: {e}"));
        assert!(out.out_dir.join("Cargo.toml").exists());
        assert!(out.out_dir.join("src/main.rs").exists());
        let cargo = std::fs::read_to_string(out.out_dir.join("Cargo.toml")).unwrap();
        let manifest = read_manifest(&root).unwrap();
        assert!(cargo.contains(&format!("name = \"{}\"", manifest.name)));
    }
}

#[test]
fn sealed_example_lock_verifies() {
    let root = example("sealed");
    verify_sealed_api(&root).expect("sealed crisp.lock must match pub API");
}

#[test]
fn math_has_tests_in_arith_module() {
    let tests = collect_tests(&example("math")).unwrap();
    assert_eq!(tests.len(), 4);
    assert_eq!(tests.iter().filter(|t| !t.compile_fail).count(), 3);
}

#[test]
fn examples_with_tests_pass_crpc_test() {
    for name in ["with_tests", "math", "defaults", "sealed"] {
        let root = example(name);
        eprintln!("crpc test: {name}");
        match run_tests(&root) {
            Ok(r) => {
                eprintln!("  runtime={} compile_fail={}", r.runtime_passed, r.compile_fail_passed);
                assert!(r.runtime_passed > 0 || r.compile_fail_passed > 0);
            }
            Err(e) if e.to_string().contains("cargo not on PATH") => {
                eprintln!("SKIP {name}: cargo not on PATH");
            }
            Err(e) => panic!("{name} tests failed: {e}"),
        }
    }
}

#[test]
fn runnable_examples_build_and_run() {
    for name in ["hello", "defaults", "sealed", "server"] {
        let root = example(name);
        eprintln!("build+run: {name}");
        match run_emitted(&root) {
            Ok(out) => {
                eprintln!("  stdout: {out:?}");
                assert!(!out.is_empty() || name == "fallible", "{name} produced no output");
            }
            Err(PipelineError::ToolchainUnavailable) => {
                eprintln!("SKIP {name}: cargo not on PATH");
            }
            Err(e) => panic!("{name} run failed: {e}"),
        }
    }
}

#[test]
fn server_emits_config_defaults() {
    let cir = CirBuilder::build_crate(&example("server")).unwrap();
    let out = crisp_rust_emit::emit_crate(&cir);
    assert!(out.modules.iter().any(|(m, _)| m == "config"));
    assert!(out.lib_rs.contains("Config::with"));
}

#[test]
fn fallible_emits_crisp_error() {
    let cir = CirBuilder::build_crate(&example("fallible")).unwrap();
    assert!(!cir.crisp_error.variants.is_empty());
    let out = crisp_rust_emit::emit_crate(&cir);
    assert!(out.lib_rs.contains("CrispError"));
    assert!(out.lib_rs.contains("Result<"));
}

fn maybe_build(name: &str) {
    let root = example(name);
    match build_emitted(&root) {
        Ok(_) => eprintln!("built {name}"),
        Err(PipelineError::ToolchainUnavailable) => eprintln!("SKIP build {name}"),
        Err(e) => panic!("build {name}: {e}"),
    }
}

/// Examples known to emit clean Rust today (fallible catch lowering is still WIP).
const BUILDABLE: &[&str] = &["hello", "server", "math", "defaults", "sealed", "with_tests"];

#[test]
fn all_runnable_examples_build() {
    for name in BUILDABLE {
        maybe_build(name);
    }
}
