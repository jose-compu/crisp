//! v1.8.1 regressions (#140–#146).

use crisp_cir::{CirBuilder, CirItem};
use crisp_rust_emit::{collect_tests, emit_crate, emit_test_module_with_cir, run_tests};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn skip_if_no_cargo(err: &crisp_rust_emit::TestHarnessError) -> bool {
    let s = err.to_string();
    s.contains("cargo not on PATH") || s.contains("not found")
}

#[test]
fn issue_142_index_cast_is_parenthesized() {
    let cir = CirBuilder::build_crate(&fixture("issue_142_index_parens")).expect("cir #142");
    let out = emit_crate(&cir);
    eprintln!("#142 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("(s * n + i) as usize") || out.lib_rs.contains("(s * n + i)as usize"),
        "index emit should parenthesize, got:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_142_runtime_at() {
    match run_tests(&fixture("issue_142_index_parens")) {
        Ok(r) => {
            eprintln!("#142 runtime_passed={}", r.runtime_passed);
            assert!(r.runtime_passed >= 1);
        }
        Err(e) if skip_if_no_cargo(&e) => {
            eprintln!("SKIP issue_142 run_tests: {e}");
        }
        Err(e) => panic!("#142 run_tests: {e}"),
    }
}

#[test]
fn issue_141_mut_index_write() {
    let cir = CirBuilder::build_crate(&fixture("issue_141_mut_index")).expect("cir #141");
    let out = emit_crate(&cir);
    eprintln!("#141 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("let mut xs"),
        "helper-returned vec should be `let mut`, got:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("mut xs:")
            || out.lib_rs.contains("&mut Vec")
            || out.lib_rs.contains("&mut xs"),
        "IndexAssign param should be mutable (`mut xs` or `&mut Vec`), got:\n{}",
        out.lib_rs
    );
    match run_tests(&fixture("issue_141_mut_index")) {
        Ok(r) => {
            eprintln!("#141 runtime_passed={}", r.runtime_passed);
            assert!(r.runtime_passed >= 2);
        }
        Err(e) if skip_if_no_cargo(&e) => {
            eprintln!("SKIP issue_141 run_tests: {e}");
        }
        Err(e) => panic!("#141 run_tests: {e}"),
    }
}

#[test]
fn issue_144_harness_len_method() {
    let tests = collect_tests(&fixture("issue_144_harness_len")).expect("collect #144");
    let cir = CirBuilder::build_crate(&fixture("issue_144_harness_len")).expect("cir #144");
    let emitted = emit_test_module_with_cir(&tests, Some(&cir));
    eprintln!("#144 tests:\n{emitted}");
    assert!(
        emitted.contains(".len()"),
        "harness should emit `.len()`, got:\n{emitted}"
    );
    assert!(
        !emitted.contains("len(&"),
        "harness must not emit `len(&`, got:\n{emitted}"
    );
    match run_tests(&fixture("issue_144_harness_len")) {
        Ok(r) => {
            eprintln!("#144 runtime_passed={}", r.runtime_passed);
            assert!(r.runtime_passed >= 1);
        }
        Err(e) if skip_if_no_cargo(&e) => {
            eprintln!("SKIP issue_144 run_tests: {e}");
        }
        Err(e) => panic!("#144 run_tests: {e}"),
    }
}

#[test]
fn issue_143_nested_private_helper_test() {
    match run_tests(&fixture("issue_143_nested_test")) {
        Ok(r) => {
            eprintln!("#143 runtime_passed={}", r.runtime_passed);
            assert!(r.runtime_passed >= 1);
        }
        Err(e) if skip_if_no_cargo(&e) => {
            eprintln!("SKIP issue_143 run_tests: {e}");
        }
        Err(e) => panic!("#143 run_tests: {e}"),
    }
}

#[test]
fn issue_145_combustion_else_typecks() {
    CirBuilder::build_crate(&fixture("issue_145_else_newline")).expect("typeck #145");
    match run_tests(&fixture("issue_145_else_newline")) {
        Ok(r) => {
            eprintln!("#145 runtime_passed={}", r.runtime_passed);
            assert!(r.runtime_passed >= 1);
        }
        Err(e) if skip_if_no_cargo(&e) => {
            eprintln!("SKIP issue_145 run_tests: {e}");
        }
        Err(e) => panic!("#145 run_tests: {e}"),
    }
}

#[test]
fn issue_146_rk4_modules() {
    let cir = CirBuilder::build_crate(&fixture("issue_146_rk4")).expect("cir #146");
    let a = cir
        .modules
        .iter()
        .find(|m| m.path == "core.a")
        .expect("core.a");
    let b = cir
        .modules
        .iter()
        .find(|m| m.path == "core.b")
        .expect("core.b");
    let step_a = a.items.iter().find_map(|i| match i {
        CirItem::Function(f) if f.name == "step_a" => Some(f),
        _ => None,
    });
    let step_b = b.items.iter().find_map(|i| match i {
        CirItem::Function(f) if f.name == "step_b" => Some(f),
        _ => None,
    });
    assert!(step_a.is_some() && step_b.is_some());
    match run_tests(&fixture("issue_146_rk4")) {
        Ok(r) => {
            eprintln!("#146 runtime_passed={}", r.runtime_passed);
            assert!(r.runtime_passed >= 1);
        }
        Err(e) if skip_if_no_cargo(&e) => {
            eprintln!("SKIP issue_146 run_tests: {e}");
        }
        Err(e) => panic!("#146 run_tests: {e}"),
    }
}
