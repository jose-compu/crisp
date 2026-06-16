//! Spec v0.2.0 abnormal / error-path audit (§7, §9, §12, §17, §19).
//!
//! Happy-path coverage lives in `conformance_e2e.rs`; this file exercises
//! rejections, compile-fail harnesses, and documents spec vs implementation deltas.

use crisp_diagnostics::format_ownership_contradiction;
use crisp_errors::{ErrorPass, ErrorPassError};
use crisp_lexer::lex;
use crisp_ownership::{OwnershipError, OwnershipPass};
use crisp_parser::Parser;
use crisp_resolve::{ResolveError, Resolver};
use crisp_rust_emit::run_tests;
use crisp_typeck::TypeChecker;
use std::fs;
use std::path::PathBuf;

fn emit_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn errors_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crisp-errors/tests/fixtures")
        .join(name)
}

fn ownership_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crisp-ownership/tests/fixtures")
        .join(name)
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

/// §7.6 / §17.4 — explicit `&` annotation contradicted by mutating use.
#[test]
fn spec_v2_s7_ownership_annotation_contradiction() {
    let root = ownership_fixture("explicit_bad");
    eprintln!("§7 explicit_bad");
    let err = OwnershipPass::analyze_crate(&root).expect_err("ownership");
    let msg = err.to_string();
    eprintln!("{msg}");
    assert!(matches!(err, OwnershipError::ContradictsAnnotation { .. }));
    // Spec §17.4 example uses E0042; implementation uses E0050 for this diagnostic.
    assert!(msg.contains("E0050"), "expected E0050, got: {msg}");
}

/// §9.2 — `!never` annotation violated by fallible body.
#[test]
fn spec_v2_s9_never_annotation_violated() {
    let root = errors_fixture("never_bad");
    eprintln!("§9 never_bad");
    let err = ErrorPass::analyze_crate(&root).expect_err("never");
    let msg = err.to_string();
    eprintln!("{msg}");
    assert!(matches!(err, ErrorPassError::NeverViolated { .. }));
    assert!(msg.contains("E0070"), "expected E0070, got: {msg}");
}

/// §9.3 — declared error set narrower than inferred reachable set.
#[test]
fn spec_v2_s9_declared_error_set_mismatch() {
    let root = errors_fixture("declared_bad");
    eprintln!("§9 declared_bad");
    let err = ErrorPass::analyze_crate(&root).expect_err("declared");
    let msg = err.to_string();
    eprintln!("{msg}");
    assert!(matches!(err, ErrorPassError::DeclaredMismatch { .. }));
    assert!(msg.contains("E0071"), "expected E0071, got: {msg}");
}

/// §3 — HM unification rejects ill-typed arithmetic.
#[test]
fn spec_v2_s3_type_mismatch_rejected() {
    let root = emit_fixture("type_mismatch");
    eprintln!("§3 type_mismatch");
    let err = TypeChecker::check_crate(&root).expect_err("typeck");
    let msg = err.to_string();
    eprintln!("{msg}");
    assert!(
        msg.contains("type mismatch") || msg.contains("unification"),
        "expected unify failure, got: {msg}"
    );
}

/// §3 — unknown callee is rejected at typecheck.
#[test]
fn spec_v2_s3_unknown_name_rejected() {
    let root = emit_fixture("unknown_name");
    eprintln!("§3 unknown_name");
    let err = TypeChecker::check_crate(&root).expect_err("typeck");
    let msg = err.to_string();
    eprintln!("{msg}");
    assert!(
        msg.contains("E0041")
            || msg.contains("E0035")
            || msg.contains("unknown")
            || msg.contains("unresolved"),
        "expected unknown name, got: {msg}"
    );
}

/// §12 — duplicate definitions in one module.
#[test]
fn spec_v2_s12_duplicate_definition_rejected() {
    let root = emit_fixture("duplicate_def");
    eprintln!("§12 duplicate_def");
    let err = Resolver::resolve_crate(&root).expect_err("resolve");
    let msg = err.to_string();
    eprintln!("{msg}");
    assert!(matches!(err, ResolveError::DuplicateDef { .. }));
    assert!(msg.contains("E0034"), "expected E0034, got: {msg}");
}

/// §12 — private symbol import rejected (E0036).
#[test]
fn spec_v2_s12_private_import_rejected() {
    let dir = std::env::temp_dir().join("crisp-spec-v2-private-import");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("crisp.toml"),
        r#"[package]
name = "priv_test"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(dir.join("src/secret.crp"), "helper() = 1\n").unwrap();
    fs::write(
        dir.join("src/main.crp"),
        "use secret { helper }\npub main() = helper()\n",
    )
    .unwrap();

    eprintln!("§12 private import");
    let err = Resolver::resolve_crate(&dir).expect_err("private");
    let msg = err.to_string();
    eprintln!("{msg}");
    assert!(matches!(
        err,
        ResolveError::PrivateImport { name, .. } if name == "helper"
    ));
    assert!(msg.contains("E0036"), "expected E0036, got: {msg}");

    let _ = fs::remove_dir_all(&dir);
}

/// §2.2 — nested block comments tokenize without panic.
#[test]
fn spec_v2_s2_nested_block_comment_lexes() {
    let src = "pub main() = 1 {- outer {- inner -} -}";
    eprintln!("§2 nested comment");
    let tokens = lex(src).expect("lex");
    assert!(tokens.len() > 3);
}

/// §4 — malformed source is a parse error, not a panic.
#[test]
fn spec_v2_s4_unclosed_block_parse_error() {
    let src = "pub main() = { print(\"hi\")";
    eprintln!("§4 unclosed block");
    let mut p = Parser::new(src).expect("parser");
    assert!(p.parse_module().is_err());
}

/// §17.4 — formatted ownership diagnostic uses implementation code E0050.
#[test]
fn spec_v2_s17_4_ownership_diagnostic_code() {
    let src = "bad(& x) = x\n";
    let d = format_ownership_contradiction(src, "x", "&mut", "&", crisp_ast::Span::new(6, 7));
    eprintln!("{}", d.rendered);
    assert!(d.rendered.contains("E0050"));
    assert!(!d.rendered.contains("E0042"));
}

/// §19 — `test_compile_fail` harness catches type-level abnormal cases.
#[test]
fn spec_v2_s19_compile_fail_harness() {
    let root = example("abnormal_suite");
    eprintln!("§19 abnormal_suite tests");
    TypeChecker::check_crate(&root).expect("main crate typechecks");
    let report = run_tests(&root).expect("run_tests");
    eprintln!(
        "runtime={} compile_fail={}",
        report.runtime_passed, report.compile_fail_passed
    );
    assert_eq!(report.runtime_passed, 1);
    assert_eq!(report.compile_fail_passed, 3);
}

/// §18 — pipeline must fail on ownership contradiction fixture.
#[test]
fn spec_v2_s18_check_fails_on_ownership_contradiction() {
    let root = ownership_fixture("explicit_bad");
    eprintln!("§18 build explicit_bad");
    let err = crisp_rust_emit::build_emitted(&root).expect_err("build");
    let msg = err.to_string();
    eprintln!("{msg}");
    assert!(
        msg.contains("E0050"),
        "expected E0050 in pipeline, got: {msg}"
    );
}

/// §12.5 — sealed lock drift remains a check failure (regression guard).
#[test]
fn spec_v2_s12_5_sealed_lock_still_valid() {
    let root = example("sealed");
    eprintln!("§12.5 sealed lock ok");
    crisp_rust_emit::verify_sealed_api(&root).expect("lock valid");
}
