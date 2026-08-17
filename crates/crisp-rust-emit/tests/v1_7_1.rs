//! v1.7.1 regression tests for #93, #94, and #96 (nested emit, powf, while/if assign).
//! Fixtures copy the GitHub reproductions so these cannot regress silently.

use crisp_cir::CirBuilder;
use crisp_rust_emit::{PipelineError, build_emitted, emit_crate, run_emitted};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn skip_or_panic(e: PipelineError, what: &str) {
    match e {
        PipelineError::ToolchainUnavailable => {
            eprintln!("SKIP {what}: cargo not on PATH");
        }
        other => panic!("{what}: {other}"),
    }
}

#[test]
fn issue_93_nested_to_nested_use_emits_crate_prefix() {
    let cir = CirBuilder::build_crate(&fixture("issue_93_nested_use")).expect("cir #93");
    let out = emit_crate(&cir);
    let double = out
        .modules
        .iter()
        .find(|(p, _)| p == "math.double")
        .map(|(_, s)| s.as_str())
        .expect("math.double.rs");
    eprintln!("#93 math/double.rs:\n{double}");
    assert!(
        double.contains("crate::math::add::add"),
        "nested use must emit crate::math::add::add (E0433):\n{double}"
    );
    assert!(
        !double.contains("\n    math::add::"),
        "must not emit a crate-root path without crate:::\n{double}"
    );
    assert!(
        out.lib_rs.contains("crate::math::double::twice"),
        "root-to-nested calls still use crate:::\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_93_nested_use_builds_and_runs() {
    let root = fixture("issue_93_nested_use");
    match build_emitted(&root) {
        Ok(_) => {}
        Err(e) => skip_or_panic(e, "issue_93 build"),
    }
    match run_emitted(&root) {
        Ok(stdout) => {
            eprintln!("#93 stdout: {stdout}");
            assert!(
                stdout.contains("6"),
                "twice(3) should print 6, got {stdout}"
            );
        }
        Err(e) => skip_or_panic(e, "issue_93 run"),
    }
}

#[test]
fn issue_94_untyped_float_powf_emits_f64() {
    let cir = CirBuilder::build_crate(&fixture("issue_94_powf")).expect("cir #94");
    let out = emit_crate(&cir);
    eprintln!("#94 main.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains(".powf("),
        "must emit powf:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("as f64"),
        "(1.0 / 0.3) ** 0.5 must not call powf on {{float}} (E0689):\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("_f64"),
        "float literals should be typed f64:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("fn pow_half") && out.lib_rs.contains("powf"),
        "annotated x: float ** 0.5 must still emit:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_94_untyped_float_powf_builds_and_runs() {
    let root = fixture("issue_94_powf");
    match build_emitted(&root) {
        Ok(_) => {}
        Err(e) => skip_or_panic(e, "issue_94 build"),
    }
    match run_emitted(&root) {
        Ok(stdout) => {
            eprintln!("#94 stdout: {stdout}");
            assert!(
                stdout.contains("1.82") || stdout.contains("1.8"),
                "sqrt(1/0.3) should print, got {stdout}"
            );
        }
        Err(e) => skip_or_panic(e, "issue_94 run"),
    }
}

#[test]
fn issue_96_bisection_emits_if_assign_and_builds() {
    let cir = CirBuilder::build_crate(&fixture("issue_96_bisection")).expect("cir #96");
    let out = emit_crate(&cir);
    eprintln!("#96 main.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("hi = mid") && out.lib_rs.contains("lo = mid"),
        "if-then assignment must lower, not drop to unit:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("(lo + hi)"),
        "bisection midpoint must keep (lo + hi) grouping (#99):\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("fn bisection_width"),
        "missing bisection_width:\n{}",
        out.lib_rs
    );
    match build_emitted(&fixture("issue_96_bisection")) {
        Ok(_) => {}
        Err(e) => skip_or_panic(e, "issue_96 build"),
    }
    match run_emitted(&fixture("issue_96_bisection")) {
        Ok(stdout) => {
            eprintln!("#96 stdout: {stdout}");
            assert!(!stdout.trim().is_empty(), "bisection should print a width");
        }
        Err(e) => skip_or_panic(e, "issue_96 run"),
    }
}
