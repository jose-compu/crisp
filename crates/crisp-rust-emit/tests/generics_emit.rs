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
    assert!(
        out.lib_rs.contains("std::ops::Add") && out.lib_rs.contains("fn distance"),
        "op bounds:\n{}",
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

#[test]
fn generics_implicit_emits_and_runs() {
    let cir = CirBuilder::build_crate(&example("generics_implicit")).expect("cir");
    let src = emit_crate(&cir).lib_rs;
    eprintln!("emitted implicit:\n{src}");
    assert!(src.contains("struct Pair<A, B>"), "Pair:\n{src}");
    assert!(src.contains("fn id<T: Clone>"), "id:\n{src}");
    assert!(src.contains("trait Boxy<T>"), "Boxy:\n{src}");
    assert!(src.contains("trait Wrapper<T>"), "Wrapper:\n{src}");
    assert!(
        src.contains("impl Wrapper<i64> for IntBox"),
        "inferred impl:\n{src}"
    );
    assert!(
        src.contains("HasName") && src.contains("HasId"),
        "shape + bound:\n{src}"
    );
    let run = run_emitted(&example("generics_implicit")).expect("run");
    assert!(run.contains("n=10"), "stdout: {run}");
    let r = run_tests(&example("generics_implicit")).expect("test");
    assert!(r.runtime_passed >= 4, "runtime_passed={}", r.runtime_passed);
    assert!(
        r.compile_fail_passed >= 1,
        "compile_fail_passed={}",
        r.compile_fail_passed
    );
}

#[test]
fn generics_pub_emits_scheme_vs_mono() {
    let cir = CirBuilder::build_crate(&example("generics_pub")).expect("cir");
    let src = emit_crate(&cir).lib_rs;
    eprintln!("emitted generics_pub:\n{src}");
    assert!(src.contains("fn id<T: Clone>"), "poly id:\n{src}");
    assert!(
        src.contains("fn once(") && !src.contains("fn once<"),
        "mono once:\n{src}"
    );
    assert!(
        src.contains("fn identity<T: Clone>"),
        "pub identity scheme:\n{src}"
    );
    let run = run_emitted(&example("generics_pub")).expect("run");
    eprintln!("stdout: {run}");
    assert!(run.contains("a=1"), "stdout: {run}");
    assert!(run.contains("b=ok"), "stdout: {run}");
    let r = run_tests(&example("generics_pub")).expect("test");
    assert!(r.runtime_passed >= 3, "runtime_passed={}", r.runtime_passed);
}

#[test]
fn pub_scheme_drift_is_e0080() {
    use crisp_manifest::CrispLock;
    use crisp_rust_emit::{update_lock, verify_sealed_api};

    let dir = tempfile::TempDir::new().unwrap();
    copy_crate(&example("generics_pub"), dir.path());
    update_lock(dir.path()).expect("lock");
    let lock_path = dir.path().join("crisp.lock");
    let mut lock: CrispLock =
        serde_json::from_str(&std::fs::read_to_string(&lock_path).unwrap()).unwrap();
    let Some(sig) = lock
        .sealed_api
        .iter_mut()
        .find(|s| s.name.contains("identity"))
    else {
        panic!("identity missing from lock: {lock:?}");
    };
    eprintln!("locked identity: {}", sig.rust_signature);
    assert!(
        sig.rust_signature.contains("<T: Clone>"),
        "lock must record the scheme: {}",
        sig.rust_signature
    );
    sig.rust_signature = "identity(x: int) -> int".into();
    std::fs::write(&lock_path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();
    let err = verify_sealed_api(dir.path()).expect_err("changing a sealed scheme must drift");
    let msg = err.to_string();
    eprintln!("E0080: {msg}");
    assert!(msg.contains("E0080"), "{msg}");
}

fn copy_crate(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    let Ok(entries) = std::fs::read_dir(src) else {
        return;
    };
    for entry in entries {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let dest = dst.join(&name);
        if entry.file_type().unwrap().is_dir() {
            copy_crate(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).unwrap();
        }
    }
}
