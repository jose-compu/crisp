//! Emit coverage for user generics and parametric shapes (#70 / #71).

use crisp_cir::CirBuilder;
use crisp_rust_emit::{emit_crate, run_emitted, run_tests};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
}

#[test]
fn generics_emits_type_fn_shape_and_trait_params() {
    let cir = CirBuilder::build_crate(&example("generics")).expect("cir");
    let out = emit_crate(&cir);
    let src = &out.lib_rs;
    eprintln!("emitted generics:\n{src}");
    assert!(src.contains("struct Pair<A, B>"), "struct Pair:\n{src}");
    assert!(src.contains("fn id<T: Clone>"), "fn id:\n{src}");
    assert!(
        src.contains("fn first<A: Clone, B: Clone>"),
        "fn first:\n{src}"
    );
    assert!(src.contains("trait Boxy<T>"), "trait Boxy:\n{src}");
    assert!(
        src.contains("impl Boxy<i64> for IntBox"),
        "impl Boxy<i64>:\n{src}"
    );
    assert!(
        src.contains("impl Boxy<String> for StrBox"),
        "impl Boxy<String>:\n{src}"
    );
    assert!(src.contains("trait Wrapper<T>"), "trait Wrapper:\n{src}");
    assert!(
        src.contains("impl Wrapper<i64> for IntBox"),
        "impl Wrapper<i64>:\n{src}"
    );
    assert!(
        src.contains("impl Wrapper<String> for StrBox"),
        "impl Wrapper<String>:\n{src}"
    );
}

#[test]
fn generics_run_and_test() {
    let out = run_emitted(&example("generics")).expect("run");
    assert!(out.contains("n=10"), "stdout: {out}");
    assert!(out.contains("mix=10"), "stdout: {out}");
    let r = run_tests(&example("generics")).expect("test");
    assert!(r.runtime_passed >= 4, "runtime_passed={}", r.runtime_passed);
    assert!(
        r.compile_fail_passed >= 1,
        "compile_fail_passed={}",
        r.compile_fail_passed
    );
}

#[test]
fn shapes_generic_emits_and_runs() {
    let cir = CirBuilder::build_crate(&example("shapes_generic")).expect("cir");
    let out = emit_crate(&cir);
    assert!(
        out.lib_rs.contains("trait Boxy<T>"),
        "emitted:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("S0: Boxy<i64>") || out.lib_rs.contains("Boxy<i64>"),
        "shape bound:\n{}",
        out.lib_rs
    );
    let run = run_emitted(&example("shapes_generic")).expect("run");
    assert!(run.contains("i=4"), "stdout: {run}");
    let r = run_tests(&example("shapes_generic")).expect("test");
    assert!(r.runtime_passed >= 1);
    assert!(
        r.compile_fail_passed >= 1,
        "compile_fail_passed={}",
        r.compile_fail_passed
    );
}
