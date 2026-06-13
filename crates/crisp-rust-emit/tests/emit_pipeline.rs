//! End-to-end emit and build tests (spec §17.1).

use crisp_cir::CirBuilder;
use crisp_rust_emit::{emit_crate, build_emitted, emit_to_target, run_emitted, PipelineError};
use std::path::PathBuf;

fn hello_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
}

fn server_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/server")
}

#[test]
fn cir_builds_hello_with_main_and_greet() {
    let cir = CirBuilder::build_crate(&hello_root()).expect("cir");
    let main = cir.modules.iter().find(|m| m.path == "main").expect("main");
    let names: Vec<_> = main
        .items
        .iter()
        .filter_map(|i| match i {
            crisp_cir::CirItem::Function(f) => Some(f.name.as_str()),
            _ => None,
        })
        .collect();
    assert!(names.contains(&"main"));
    assert!(names.contains(&"greet"));
}

#[test]
fn emit_hello_contains_println_and_greet() {
    let cir = CirBuilder::build_crate(&hello_root()).expect("cir");
    let out = emit_crate(&cir);
    assert!(out.lib_rs.contains("fn greet"));
    assert!(out.lib_rs.contains("fn main"));
    assert!(out.lib_rs.contains("println!"));
    assert!(out.lib_rs.contains("format!"));
    insta::assert_snapshot!("emit_hello_main", out.lib_rs);
}

#[test]
fn emit_server_config_with_defaults() {
    let cir = CirBuilder::build_crate(&server_root()).expect("cir");
    let out = emit_crate(&cir);
    assert!(out.lib_rs.contains("mod config"));
    assert!(out.lib_rs.contains("Config::with"));
    let config_src = out
        .modules
        .iter()
        .find(|(m, _)| m == "config")
        .map(|(_, s)| s.as_str())
        .expect("config.rs");
    assert!(config_src.contains("localhost"));
    assert!(config_src.contains("pub fn with"));
}

#[test]
fn emit_writes_target_rust_project() {
    let root = hello_root();
    let out = emit_to_target(&root).expect("emit");
    assert!(out.out_dir.join("Cargo.toml").exists());
    assert!(out.out_dir.join("src/main.rs").exists());
    assert!(out.main_rs.contains("fn main"));
}

#[test]
fn hello_builds_with_cargo() {
    let root = hello_root();
    match build_emitted(&root) {
        Ok(dir) => {
            assert!(dir.join("target/debug").exists() || dir.join("target").exists());
        }
        Err(PipelineError::ToolchainUnavailable) => {
            eprintln!("SKIP hello_builds_with_cargo: cargo not on PATH");
        }
        Err(e) => panic!("hello should build: {e}"),
    }
}

#[test]
fn hello_run_prints_greeting() {
    let root = hello_root();
    match run_emitted(&root) {
        Ok(stdout) => {
            assert!(stdout.contains("hello crisp"), "stdout: {stdout}");
        }
        Err(PipelineError::ToolchainUnavailable) => {
            eprintln!("SKIP hello_run_prints_greeting: cargo not on PATH");
        }
        Err(e) => panic!("hello should run: {e}"),
    }
}

#[test]
fn ice_mapper_parses_rustc_line() {
    use crisp_rust_emit::EmitSourceMap;
    let mut map = EmitSourceMap::default();
    map.record(0, crisp_ast::Span::new(10, 20));
    let src = "line1\nline2\n";
    assert!(map.lookup_line(1, src).is_some());
}
