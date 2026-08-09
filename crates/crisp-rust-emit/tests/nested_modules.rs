//! Nested module emit (`src/math/vector.crp`) — issue #35.

use crisp_cir::CirBuilder;
use crisp_rust_emit::{PipelineError, build_emitted, emit_crate, emit_to_target};
use std::path::PathBuf;

fn nested_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/nested_math")
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
    let math = out
        .modules
        .iter()
        .find(|(p, _)| p == "math")
        .map(|(_, s)| s.as_str())
        .expect("parent math module source");
    assert!(math.contains("pub mod vector;"), "math.rs:\n{math}");
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
