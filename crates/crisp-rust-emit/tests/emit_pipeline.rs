//! End-to-end emit and build tests (spec §17.1).
//!
//! #25 pins CIR outlines and Rust emit for `examples/hello` and `examples/math`.
//! Update after intentional emit changes:
//! `INSTA_UPDATE=1 cargo test -p crisp-rust-emit --test emit_pipeline`

use crisp_cir::{CirBuilder, CirCrate, CirItem};
use crisp_rust_emit::{
    EmitResult, PipelineError, build_emitted, emit_crate, emit_to_target, run_emitted,
};
use std::fmt::Write;
use std::path::PathBuf;

fn hello_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
}

fn server_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/server")
}

fn math_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/math")
}

fn defaults_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/defaults")
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
            assert!(stdout.contains("hello world"), "stdout: {stdout}");
        }
        Err(PipelineError::ToolchainUnavailable) => {
            eprintln!("SKIP hello_run_prints_greeting: cargo not on PATH");
        }
        Err(e) => panic!("hello should run: {e}"),
    }
}

#[test]
fn server_builds_with_cargo() {
    let root = server_root();
    match build_emitted(&root) {
        Ok(_) => {}
        Err(PipelineError::ToolchainUnavailable) => {
            eprintln!("SKIP server_builds_with_cargo: cargo not on PATH");
        }
        Err(e) => panic!("server should build: {e}"),
    }
}

#[test]
fn math_emits_arith_module() {
    let cir = CirBuilder::build_crate(&math_root()).expect("cir");
    let out = emit_crate(&cir);
    assert!(out.lib_rs.contains("mod arith"));
    assert!(
        out.modules
            .iter()
            .any(|(m, s)| m == "arith" && s.contains("fn product"))
    );
    insta::assert_snapshot!("emit_math_bundle", emit_bundle(&out));
}

#[test]
fn cir_outline_hello_snapshot() {
    let cir = CirBuilder::build_crate(&hello_root()).expect("cir");
    insta::assert_snapshot!("cir_outline_hello", cir_outline(&cir));
}

#[test]
fn cir_outline_math_snapshot() {
    let cir = CirBuilder::build_crate(&math_root()).expect("cir");
    insta::assert_snapshot!("cir_outline_math", cir_outline(&cir));
}

/// Stable, span-free CIR outline for conformance pinning (#25).
fn cir_outline(cir: &CirCrate) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "package {}", cir.package_name);
    let mut modules = cir.modules.clone();
    modules.sort_by(|a, b| a.path.cmp(&b.path));
    for m in modules {
        let _ = writeln!(out, "module {}", m.path);
        for item in &m.items {
            match item {
                CirItem::Struct(s) => {
                    let fields: Vec<_> = s.fields.iter().map(|f| f.name.as_str()).collect();
                    let _ = writeln!(out, "  struct {} {{{}}}", s.name, fields.join(", "));
                }
                CirItem::Enum(e) => {
                    let vars: Vec<_> = e.variants.iter().map(|v| v.name.as_str()).collect();
                    let _ = writeln!(out, "  enum {} {{{}}}", e.name, vars.join(", "));
                }
                CirItem::Alias { name, .. } => {
                    let _ = writeln!(out, "  alias {name}");
                }
                CirItem::Function(f) => {
                    let params: Vec<_> = f.params.iter().map(|p| p.name.as_str()).collect();
                    let _ = writeln!(out, "  fn {}({})", f.name, params.join(", "));
                }
                CirItem::Trait(t) => {
                    let _ = writeln!(out, "  trait {}", t.name);
                }
                CirItem::Impl(i) => {
                    let _ = writeln!(out, "  impl {}", i.trait_name.as_deref().unwrap_or("_"));
                }
                CirItem::Extern(ext) => {
                    let _ = writeln!(out, "  extern ({})", ext.functions.len());
                }
            }
        }
    }
    out
}

fn emit_bundle(out: &EmitResult) -> String {
    let mut buf = String::new();
    let _ = writeln!(buf, "// === main / lib.rs ===");
    buf.push_str(&out.lib_rs);
    if !buf.ends_with('\n') {
        buf.push('\n');
    }
    let mut mods = out.modules.clone();
    mods.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, src) in mods {
        let _ = writeln!(buf, "// === mod {name} ===");
        buf.push_str(&src);
        if !buf.ends_with('\n') {
            buf.push('\n');
        }
    }
    buf
}

#[test]
fn defaults_emits_with_builder() {
    let cir = CirBuilder::build_crate(&defaults_root()).expect("cir");
    let out = emit_crate(&cir);
    assert!(out.lib_rs.contains("ServerConfig::with"));
    assert!(out.lib_rs.contains("127.0.0.1"));
}

#[test]
fn ice_mapper_parses_rustc_line() {
    use crisp_rust_emit::EmitSourceMap;
    let mut map = EmitSourceMap::default();
    map.record(0, crisp_ast::Span::new(10, 20));
    let src = "line1\nline2\n";
    assert!(map.lookup_line(1, src).is_some());
}
