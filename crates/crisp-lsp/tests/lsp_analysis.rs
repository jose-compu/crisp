//! LSP analysis integration tests (spec §16.3).

use crisp_lsp::CrispAnalysis;
use std::path::{Path, PathBuf};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn main_crp(root: &Path) -> PathBuf {
    root.join("src/main.crp")
}

#[test]
fn analyze_hello_crate() {
    let root = example("hello");
    eprintln!("analyze hello");
    let a = CrispAnalysis::analyze(&root).expect("analyze");
    assert!(!a.typed.signatures.is_empty());
}

#[test]
fn hover_greet_name_in_hello() {
    let root = example("hello");
    let file = main_crp(&root);
    let src = std::fs::read_to_string(&file).unwrap();
    let offset = src.find("greet(").expect("greet call") as u32 + 1;
    let a = CrispAnalysis::analyze(&root).unwrap();
    let hover = a.hover(&file, offset).unwrap().expect("hover");
    eprintln!("hover: {:?}", hover);
    assert!(hover.markdown.contains("greet") || hover.title == "greet");
}

#[test]
fn inlay_hints_hello_bindings() {
    let root = example("hello");
    let file = main_crp(&root);
    let a = CrispAnalysis::analyze(&root).unwrap();
    let hints = a.inlay_hints(&file).unwrap();
    eprintln!("hints: {hints:?}");
    assert!(!hints.is_empty());
}

#[test]
fn call_overlays_fallible_read_config() {
    let root = example("fallible");
    let file = main_crp(&root);
    let a = CrispAnalysis::analyze(&root).unwrap();
    let overlays = a.call_overlays(&file).unwrap();
    eprintln!("overlays: {overlays:?}");
    assert!(
        overlays
            .iter()
            .any(|o| o.callee == "read_config" && o.fallible)
    );
}

#[test]
fn code_lenses_hello_show_rust() {
    let root = example("hello");
    let file = main_crp(&root);
    let a = CrispAnalysis::analyze(&root).unwrap();
    let lenses = a.code_lenses(&file).unwrap();
    eprintln!("lenses: {lenses:?}");
    assert!(lenses.iter().any(|l| l.title == "Show emitted Rust"));
    assert!(lenses.iter().any(|l| l.title == "Run crpc test"));
}

#[test]
fn emitted_rust_hello_has_main() {
    let root = example("hello");
    let a = CrispAnalysis::analyze(&root).unwrap();
    let rust = a.emitted_rust().expect("emit");
    eprintln!("rust len={}", rust.len());
    assert!(rust.contains("fn main"));
    assert!(rust.contains("fn greet"));
}

#[test]
fn hover_server_config_field() {
    let root = example("server");
    let config = root.join("src/config.crp");
    let src = std::fs::read_to_string(&config).unwrap();
    let offset = src.find("host").expect("host field") as u32;
    let a = CrispAnalysis::analyze(&root).unwrap();
    let hover = a.hover(&config, offset);
    eprintln!("config hover: {hover:?}");
    let _ = hover;
}

#[test]
fn analyze_all_examples() {
    for name in [
        "hello",
        "server",
        "fallible",
        "math",
        "match",
        "async_hello",
        "ffi",
        "stdlib_smoke",
        "patterns",
        "kitchen_sink",
        "ownership_demo",
        "inventory",
        "vec_ops",
        "fallible_chain",
        "async_spawn",
        "workshop",
        "unsafe_math",
        "data_pipeline",
        "abnormal_suite",
    ] {
        eprintln!("analyze {name}");
        let root = example(name);
        CrispAnalysis::analyze(&root).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}
