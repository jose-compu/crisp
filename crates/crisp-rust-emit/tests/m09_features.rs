//! Milestone 0.9 feature tests: match, async, ffi, stdlib, fallible.

use crisp_cir::CirBuilder;
use crisp_rust_emit::{build_emitted, emit_crate, run_emitted, PipelineError};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(name)
}

#[test]
fn m09_fallible_builds_and_runs() {
    let root = example("fallible");
    eprintln!("fallible build");
    let out = run_emitted(&root).unwrap_or_else(|e| panic!("fallible: {e}"));
    eprintln!("fallible stdout: {out:?}");
    assert!(out.contains("3000"));
}

#[test]
fn m09_match_emits_match_expression() {
    let cir = CirBuilder::build_crate(&example("match")).expect("cir");
    let out = emit_crate(&cir);
    eprintln!("match emit:\n{}", out.lib_rs);
    assert!(out.lib_rs.contains("match "));
    assert!(out.lib_rs.contains("=>"));
    let run = run_emitted(&example("match")).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("match run: {run:?}");
    assert!(run.contains("matched"));
}

#[test]
fn m09_async_emits_tokio_main() {
    let cir = CirBuilder::build_crate(&example("async_hello")).expect("cir");
    let main_fn = cir
        .modules
        .iter()
        .find(|m| m.path == "main")
        .and_then(|m| m.items.iter().find_map(|i| match i {
            crisp_cir::CirItem::Function(f) if f.is_main => Some(f),
            _ => None,
        }))
        .expect("main");
    assert!(main_fn.is_async);
    let out = emit_crate(&cir);
    eprintln!("async emit:\n{}", out.lib_rs);
    assert!(out.lib_rs.contains("#[tokio::main]"));
    assert!(out.lib_rs.contains("async fn main"));
    assert!(out.lib_rs.contains("tokio::time::sleep"));
    let run = run_emitted(&example("async_hello")).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("async run: {run:?}");
    assert_eq!(run.trim(), "async-ok");
}

#[test]
fn m09_ffi_emits_extern_and_unsafe() {
    let cir = CirBuilder::build_crate(&example("ffi")).expect("cir");
    let out = emit_crate(&cir);
    eprintln!("ffi emit:\n{}", out.lib_rs);
    assert!(out.lib_rs.contains("extern \"C\""));
    assert!(out.lib_rs.contains("unsafe"));
    assert!(out.lib_rs.contains("fn abs"));
    let run = run_emitted(&example("ffi")).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("ffi run: {run:?}");
    assert!(run.contains("ffi-result=7"));
}

#[test]
fn m09_stdlib_smoke_vec_and_test() {
    let cir = CirBuilder::build_crate(&example("stdlib_smoke")).expect("cir");
    let out = emit_crate(&cir);
    eprintln!("stdlib emit:\n{}", out.lib_rs);
    assert!(out.lib_rs.contains("Vec::<i64>::new()"));
    assert!(out.lib_rs.contains(".len()"));
    let run = run_emitted(&example("stdlib_smoke")).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("stdlib run: {run:?}");
    assert!(run.contains("empty-len=0"));
}

#[test]
fn m09_all_new_examples_build() {
    for name in ["match", "async_hello", "ffi", "stdlib_smoke", "fallible"] {
        eprintln!("build {name}");
        match build_emitted(&example(name)) {
            Ok(_) => eprintln!("  ok"),
            Err(PipelineError::ToolchainUnavailable) => eprintln!("  SKIP (no cargo)"),
            Err(e) => panic!("{name}: {e}"),
        }
    }
}

#[test]
fn m09_cir_has_extern_item_for_ffi() {
    let cir = CirBuilder::build_crate(&example("ffi")).expect("cir");
    let has_extern = cir.modules.iter().any(|m| {
        m.items.iter().any(|i| matches!(i, crisp_cir::CirItem::Extern(_)))
    });
    assert!(has_extern);
}
