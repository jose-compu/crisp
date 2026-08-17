//! Nested module emit (`src/math/vector.crp`) — issue #35.

use crisp_cir::CirBuilder;
use crisp_resolve::Resolver;
use crisp_rust_emit::{PipelineError, build_emitted, emit_crate, emit_to_target, run_emitted};
use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn nested_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/nested_math")
}

fn deep_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/nested_deep")
}

#[test]
fn nested_math_resolves_and_typechecks() {
    Resolver::resolve_crate(&nested_root()).expect("resolve");
    TypeChecker::check_crate(&nested_root()).expect("typeck");
}

#[test]
fn nested_emit_declares_parent_and_child_mods() {
    let cir = CirBuilder::build_crate(&nested_root()).expect("cir");
    assert!(
        cir.modules.iter().any(|m| m.path == "math.vector"),
        "resolve should load math.vector"
    );
    let out = emit_crate(&cir);
    assert!(
        out.lib_rs.contains("mod math;"),
        "crate root must declare top-level math, got:\n{}",
        out.lib_rs
    );
    assert!(
        !out.lib_rs.contains("mod math.vector"),
        "must not emit invalid dotted mod"
    );
    assert!(
        out.lib_rs.contains("crate::math::vector::") || out.lib_rs.contains("origin"),
        "intra-crate paths need crate:: so nested modules compile (#93):\n{}",
        out.lib_rs
    );
    assert!(
        !out.lib_rs.contains("math.vector::"),
        "must not use Crisp dots in Rust paths:\n{}",
        out.lib_rs
    );
    let math = out
        .modules
        .iter()
        .find(|(p, _)| p == "math")
        .map(|(_, s)| s.as_str())
        .expect("parent math module source");
    assert!(math.contains("pub mod vector;"), "math.rs:\n{math}");
    assert!(math.contains("pub mod double;"), "math.rs:\n{math}");
    assert!(math.contains("pub mod scale;"), "math.rs:\n{math}");
    let vector = out
        .modules
        .iter()
        .find(|(p, _)| p == "math.vector")
        .map(|(_, s)| s.as_str())
        .expect("math.vector source");
    assert!(
        vector.contains("fn origin") || vector.contains("struct Vec2"),
        "{vector}"
    );
    let double = out
        .modules
        .iter()
        .find(|(p, _)| p == "math.double")
        .map(|(_, s)| s.as_str())
        .expect("math.double source");
    assert!(
        double.contains("crate::math::scale::scale"),
        "nested-to-nested use must emit crate:: (#93):\n{double}"
    );
    assert!(
        double.contains("crate::math::vector::Vec2"),
        "nested type use must emit crate::math::vector::Vec2 (#100):\n{double}"
    );
    assert!(
        !double.contains("fn twice<"),
        "twice must infer float from scale, not generalize to T: Clone:\n{double}"
    );
    insta::assert_snapshot!("emit_nested_math_main", out.lib_rs);
}

#[test]
fn nested_project_writes_nested_rust_files() {
    let root = nested_root();
    let out = emit_to_target(&root).expect("emit");
    assert!(out.out_dir.join("src/main.rs").exists());
    assert!(
        out.out_dir.join("src/math.rs").exists(),
        "expected src/math.rs"
    );
    assert!(
        out.out_dir.join("src/math/vector.rs").exists(),
        "expected src/math/vector.rs"
    );
}

#[test]
fn nested_math_builds_with_cargo() {
    match build_emitted(&nested_root()) {
        Ok(_) => {}
        Err(PipelineError::ToolchainUnavailable) => {
            eprintln!("SKIP nested_math_builds_with_cargo: cargo not on PATH");
        }
        Err(e) => panic!("nested_math should build: {e}"),
    }
}

#[test]
fn nested_math_run_prints_sum() {
    match run_emitted(&nested_root()) {
        Ok(stdout) => {
            assert!(stdout.contains("sum=3"), "stdout: {stdout}");
            assert!(stdout.contains("twice=6"), "stdout: {stdout}");
        }
        Err(PipelineError::ToolchainUnavailable) => {
            eprintln!("SKIP nested_math_run_prints_sum: cargo not on PATH");
        }
        Err(e) => panic!("nested_math should run: {e}"),
    }
}

#[test]
fn deep_nested_geo_math_vector_emits_three_levels() {
    let cir = CirBuilder::build_crate(&deep_root()).expect("cir");
    assert!(cir.modules.iter().any(|m| m.path == "geo.math.vector"));
    let out = emit_crate(&cir);
    assert!(out.lib_rs.contains("mod geo;"));
    assert!(!out.lib_rs.contains("mod geo.math"));
    let paths: Vec<_> = out.modules.iter().map(|(p, _)| p.as_str()).collect();
    assert!(paths.contains(&"geo"), "{paths:?}");
    assert!(paths.contains(&"geo.math"), "{paths:?}");
    assert!(paths.contains(&"geo.math.vector"), "{paths:?}");

    let written = emit_to_target(&deep_root()).expect("emit");
    assert!(written.out_dir.join("src/geo.rs").exists());
    assert!(written.out_dir.join("src/geo/math.rs").exists());
    assert!(written.out_dir.join("src/geo/math/vector.rs").exists());

    match build_emitted(&deep_root()) {
        Ok(_) => {}
        Err(PipelineError::ToolchainUnavailable) => {
            eprintln!("SKIP deep nested build: cargo not on PATH");
        }
        Err(e) => panic!("nested_deep should build: {e}"),
    }
}

#[test]
fn flat_math_example_still_emits_sibling_mods() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/math");
    let out = emit_crate(&CirBuilder::build_crate(&root).expect("cir"));
    assert!(out.lib_rs.contains("mod arith;"));
    assert!(out.lib_rs.contains("mod floats;"));
    assert!(out.modules.iter().any(|(p, _)| p == "arith"));
    assert!(out.modules.iter().any(|(p, _)| p == "floats"));
}
