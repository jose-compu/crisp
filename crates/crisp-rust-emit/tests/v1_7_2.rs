//! v1.7.2 regression tests for #99–#102 (parens, nested types, string match, test harness).

use crisp_cir::{CirBuilder, CirExpr, CirItem, CirPat};
use crisp_rust_emit::{
    PipelineError, build_emitted, collect_tests, emit_crate, emit_test_module, run_emitted,
    run_tests,
};
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

fn match_pats<'a>(expr: &'a CirExpr, out: &mut Vec<&'a CirPat>) {
    match expr {
        CirExpr::Match { arms, .. } => {
            for arm in arms {
                out.push(&arm.pat);
            }
        }
        CirExpr::Block(b) => {
            if let Some(t) = &b.tail {
                match_pats(t, out);
            }
        }
        CirExpr::If {
            then_branch,
            else_branch,
            ..
        } => {
            match_pats(then_branch, out);
            if let Some(e) = else_branch {
                match_pats(e, out);
            }
        }
        _ => {}
    }
}

#[test]
fn issue_101_string_match_keeps_literals() {
    let cir = CirBuilder::build_crate(&fixture("issue_101_string_match")).expect("cir #101");
    let main = cir.modules.iter().find(|m| m.path == "main").expect("main");
    let parse = main
        .items
        .iter()
        .find_map(|i| match i {
            CirItem::Function(f) if f.name == "parse" => Some(f),
            _ => None,
        })
        .expect("parse");
    let mut pats = Vec::new();
    if let Some(tail) = &parse.body.tail {
        match_pats(tail, &mut pats);
    }
    eprintln!("#101 pats: {pats:?}");
    assert!(
        pats.iter()
            .any(|p| matches!(p, CirPat::Str { value, .. } if value == "h2")),
        "expected CirPat::Str h2, got {pats:?}"
    );
    assert!(
        pats.iter()
            .any(|p| matches!(p, CirPat::Str { value, .. } if value == "ch4")),
        "expected CirPat::Str ch4, got {pats:?}"
    );
    assert!(
        !pats.iter().all(|p| matches!(p, CirPat::Wildcard { .. })),
        "string arms must not all be wildcards: {pats:?}"
    );

    let out = emit_crate(&cir);
    eprintln!("#101 main.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("\"ch4\"") && out.lib_rs.contains("\"h2\""),
        "string arms must appear in emit:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("AsRef::<str>::as_ref") || out.lib_rs.contains(".as_str()"),
        "string match must borrow as &str:\n{}",
        out.lib_rs
    );
    assert!(
        !out.lib_rs.contains("_ => 1"),
        "first string arm must not be wildcard:\n{}",
        out.lib_rs
    );

    match build_emitted(&fixture("issue_101_string_match")) {
        Ok(_) => {}
        Err(e) => skip_or_panic(e, "issue_101 build"),
    }
    match run_emitted(&fixture("issue_101_string_match")) {
        Ok(stdout) => {
            eprintln!("#101 stdout: {stdout}");
            assert!(
                stdout.trim().contains('2'),
                "parse(\"ch4\") should print 2, got {stdout}"
            );
        }
        Err(e) => skip_or_panic(e, "issue_101 run"),
    }
}

#[test]
fn issue_99_parens_preserved() {
    let cir = CirBuilder::build_crate(&fixture("issue_99_parens")).expect("cir #99");
    let out = emit_crate(&cir);
    eprintln!("#99 main.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("(lo + hi)") || out.lib_rs.contains("(lo + hi)"),
        "(lo + hi) / 2 must keep grouping:\n{}",
        out.lib_rs
    );
    assert!(
        !out.lib_rs.contains("lo + hi / 2"),
        "must not emit unparenthesized lo + hi / 2:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("(a + b)") && out.lib_rs.contains("(c - d)"),
        "(a + b) * (c - d) must keep both groups:\n{}",
        out.lib_rs
    );

    match build_emitted(&fixture("issue_99_parens")) {
        Ok(_) => {}
        Err(e) => skip_or_panic(e, "issue_99 build"),
    }
    match run_emitted(&fixture("issue_99_parens")) {
        Ok(stdout) => {
            eprintln!("#99 stdout: {stdout}");
            let nums: Vec<&str> = stdout.split_whitespace().collect();
            assert!(
                nums.iter().any(|n| n.starts_with('6')),
                "mid(2, 10) should print 6, got {stdout}"
            );
            assert!(
                nums.iter().any(|n| n.starts_with("12")),
                "grouped(1,2,5,1) should print 12, got {stdout}"
            );
        }
        Err(e) => skip_or_panic(e, "issue_99 run"),
    }
}

#[test]
fn issue_100_nested_type_use_emits_crate_prefix() {
    let cir = CirBuilder::build_crate(&fixture("issue_100_nested_type")).expect("cir #100");
    let out = emit_crate(&cir);
    let b = out
        .modules
        .iter()
        .find(|(p, _)| p == "fail.b")
        .map(|(_, s)| s.as_str())
        .expect("fail/b.rs");
    eprintln!("#100 fail/b.rs:\n{b}");
    assert!(
        b.contains("crate::fail::a::Verdict"),
        "nested type use must emit crate::fail::a::Verdict (E0425):\n{b}"
    );
    let a = out
        .modules
        .iter()
        .find(|(p, _)| p == "fail.a")
        .map(|(_, s)| s.as_str())
        .expect("fail/a.rs");
    eprintln!("#100 fail/a.rs:\n{a}");
    assert!(
        a.contains("enum Verdict") || a.contains("Verdict::Ignition"),
        "defining module still emits Verdict:\n{a}"
    );

    match build_emitted(&fixture("issue_100_nested_type")) {
        Ok(_) => {}
        Err(e) => skip_or_panic(e, "issue_100 build"),
    }
    match run_emitted(&fixture("issue_100_nested_type")) {
        Ok(stdout) => {
            eprintln!("#100 stdout: {stdout}");
            assert!(
                stdout.contains("Ignition"),
                "should print Ignition, got {stdout}"
            );
        }
        Err(e) => skip_or_panic(e, "issue_100 run"),
    }
}

#[test]
fn issue_102_harness_names_assert_and_show() {
    let root = fixture("issue_102_harness");
    let tests = collect_tests(&root).expect("collect #102");
    let emitted = emit_test_module(&tests);
    eprintln!("#102 emitted tests:\n{emitted}");
    assert!(
        emitted.contains("fn test_analysis_ignition_wide_kernel_ignites"),
        "{emitted}"
    );
    assert!(
        emitted.contains("fn test_failure_relight_wide_kernel_ignites"),
        "{emitted}"
    );
    assert!(
        emitted.contains("assert_eq!") && emitted.contains("true"),
        "bool assert_eq must not use .abs():\n{emitted}"
    );
    assert!(
        emitted.contains("FLASHBACK") && emitted.contains("assert_eq!"),
        "str assert_eq must compile as assert_eq!:\n{emitted}"
    );
    assert!(
        emitted.contains(".abs() < 1e-9"),
        "float assert_eq still uses epsilon:\n{emitted}"
    );

    let cir = CirBuilder::build_crate(&root).expect("cir #102");
    let out = emit_crate(&cir);
    let line = out
        .modules
        .iter()
        .find(|(p, _)| p == "traits.line")
        .map(|(_, s)| s.as_str())
        .expect("traits/line.rs");
    eprintln!("#102 traits/line.rs:\n{line}");
    assert!(
        line.contains("pub trait Show"),
        "nested Show must be pub so tests can see it:\n{line}"
    );

    match run_tests(&root) {
        Ok(report) => {
            eprintln!(
                "#102 runtime_passed={} compile_fail_passed={}",
                report.runtime_passed, report.compile_fail_passed
            );
            assert!(
                report.runtime_passed >= 4,
                "expected duplicate-title + float + show tests, got {}",
                report.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_102 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_102 harness: {e}"),
    }
}
