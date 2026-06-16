//! Spec-section conformance matrix (milestone 1.0).
//!
//! Each test maps to a spec section and exercises one pipeline stage end-to-end.

use crisp_cir::CirBuilder;
use crisp_diagnostics::{format_ownership_contradiction, format_type_mismatch};
use crisp_lexer::lex;
use crisp_lsp::CrispAnalysis;
use crisp_parser::Parser;
use crisp_resolve::Resolver;
use crisp_rust_emit::{PipelineError, emit_crate, emit_to_target, run_emitted};
use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

/// §2 — Lexer tokenization
#[test]
fn spec_s2_lexer_tokens() {
    let src = "pub main() = print(\"hi\")";
    let tokens = lex(src).expect("lex");
    eprintln!("§2 tokens: {}", tokens.len());
    assert!(tokens.len() > 4);
}

/// §3.4 — Type inference (HM)
#[test]
fn spec_s3_type_inference() {
    let root = example("math");
    eprintln!("§3.4 typecheck math");
    let typed = TypeChecker::check_crate(&root).expect("typeck");
    assert!(typed.signatures.values().any(|s| s.name == "main"));
}

/// §4 — Expressions (match, if)
#[test]
fn spec_s4_expressions_patterns() {
    let root = example("patterns");
    eprintln!("§4 patterns");
    TypeChecker::check_crate(&root).expect("typeck");
    let cir = CirBuilder::build_crate(&root).expect("cir");
    let out = emit_crate(&cir);
    assert!(out.lib_rs.contains("match "));
}

/// §7 — Ownership pass
#[test]
fn spec_s7_ownership_demo() {
    let root = example("ownership_demo");
    eprintln!("§7 ownership_demo");
    CrispAnalysis::analyze(&root).expect("analyze");
}

/// §8–§9 — Regions + reachable errors
#[test]
fn spec_s8_s9_fallible_errors() {
    let root = example("fallible");
    eprintln!("§8–§9 fallible");
    let a = CrispAnalysis::analyze(&root).expect("analyze");
    let overlays = a
        .call_overlays(&root.join("src/main.crp"))
        .expect("overlays");
    assert!(overlays.iter().any(|o| o.fallible));
}

/// §12 — Name resolution + modules
#[test]
fn spec_s12_multi_module_server() {
    let root = example("server");
    eprintln!("§12 server resolve");
    let resolved = Resolver::resolve_crate(&root).expect("resolve");
    assert!(resolved.modules.iter().any(|m| m.module_path == "config"));
    assert!(resolved.modules.iter().any(|m| m.module_path == "greet"));
}

/// §12.5 — Sealed API / lockfile
#[test]
fn spec_s12_5_sealed_lock() {
    let root = example("sealed");
    eprintln!("§12.5 sealed");
    crisp_rust_emit::verify_sealed_api(&root).expect("lock");
}

/// §16.3 — LSP analysis layer
#[test]
fn spec_s16_3_lsp_hover_and_lenses() {
    let root = example("hello");
    let file = root.join("src/main.crp");
    let a = CrispAnalysis::analyze(&root).expect("analyze");
    let hints = a.inlay_hints(&file).expect("hints");
    let lenses = a.code_lenses(&file).expect("lenses");
    eprintln!("§16.3 hints={} lenses={}", hints.len(), lenses.len());
    assert!(!hints.is_empty());
    assert!(lenses.iter().any(|l| l.title == "Show emitted Rust"));
}

/// §17.1 — CIR + Rust emission
#[test]
fn spec_s17_1_emit_kitchen_sink() {
    let root = example("kitchen_sink");
    eprintln!("§17.1 kitchen_sink emit");
    let out = emit_to_target(&root).expect("emit");
    assert!(out.out_dir.join("src/main.rs").exists());
    let rust = std::fs::read_to_string(out.out_dir.join("src/main.rs")).unwrap();
    assert!(rust.contains("pub fn main") || rust.contains("fn main"));
    assert!(rust.contains("Vec::<i64>::new()"));
}

/// §17.4 — Diagnostic formatting
#[test]
fn spec_s17_4_formatted_diagnostics() {
    let src = "let x := 1\n";
    let d = format_type_mismatch(src, "str", "int", crisp_ast::Span::new(9, 10));
    eprintln!("{}", d.rendered);
    assert!(d.rendered.contains("E0041"));
    let o = format_ownership_contradiction(src, "x", "own", "&", crisp_ast::Span::new(4, 5));
    assert!(o.rendered.contains("E0050"));
}

/// §18 — CLI pipeline (emit + run)
#[test]
fn spec_s18_kitchen_sink_runs() {
    let root = example("kitchen_sink");
    eprintln!("§18 kitchen_sink run");
    match run_emitted(&root) {
        Ok(out) => {
            eprintln!("stdout: {out:?}");
            assert!(out.contains("port=3000"));
            assert!(out.contains("len=0"));
        }
        Err(PipelineError::ToolchainUnavailable) => eprintln!("SKIP: cargo not on PATH"),
        Err(e) => panic!("run: {e}"),
    }
}

/// §20 — Integrated examples corpus
#[test]
fn spec_s20_all_examples_typecheck() {
    for name in [
        "hello",
        "server",
        "fallible",
        "math",
        "defaults",
        "sealed",
        "with_tests",
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
        "design_patterns",
    ] {
        eprintln!("§20 typecheck {name}");
        TypeChecker::check_crate(&example(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

/// Appendix A — Parser surface
#[test]
fn spec_appendix_a_parse_fragments() {
    let src = r#"
type T = { a: int }
greet(x) = x
pub main() = match 1 { n -> n }
"#;
    eprintln!("Appendix A parse");
    let mut p = Parser::new(src).expect("parser");
    p.parse_module().expect("module");
}
