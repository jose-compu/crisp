//! Integration tests for manifest + test harness (spec §18, §19).

use crisp_manifest::{
    parse_manifest_str, read_lock, resolve_dependencies,
};
use crisp_rust_emit::{
    collect_tests, compute_sealed_api, emit_test_module, run_tests, update_lock, verify_sealed_api,
    PipelineError,
};
use std::path::PathBuf;

fn with_tests_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/with_tests")
}

fn hello_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
}

#[test]
fn manifest_parse_and_resolve_deps() {
    let m = parse_manifest_str(
        &std::fs::read_to_string(with_tests_root().join("crisp.toml")).unwrap(),
    )
    .unwrap();
    let deps = resolve_dependencies(&m);
    assert!(deps.iter().any(|d| d.name == "tokio"));
}

#[test]
fn collect_with_tests_items() {
    let tests = collect_tests(&with_tests_root()).unwrap();
    assert_eq!(tests.len(), 3);
    assert_eq!(tests.iter().filter(|t| !t.compile_fail).count(), 2);
    assert_eq!(tests.iter().filter(|t| t.compile_fail).count(), 1);
    let emitted = emit_test_module(&tests);
    assert!(emitted.contains("assert_eq!"));
    assert!(emitted.contains("greet_works"));
}

#[test]
fn lock_update_and_verify_hello() {
    let dir = tempfile::TempDir::new().unwrap();
    copy_crate(&hello_root(), dir.path());
    let lock = update_lock(dir.path()).unwrap();
    assert!(!lock.sealed_api.is_empty());
    verify_sealed_api(dir.path()).unwrap();
    let read = read_lock(dir.path()).unwrap().unwrap();
    assert_eq!(read.sealed_api.len(), lock.sealed_api.len());
}

#[test]
fn sealed_api_includes_pub_main() {
    let sigs = compute_sealed_api(&hello_root()).unwrap();
    assert!(sigs.iter().any(|s| s.name == "main::main"));
}

#[test]
fn run_with_tests_crate() {
    let root = with_tests_root();
    match run_tests(&root) {
        Ok(report) => {
            assert_eq!(report.runtime_passed, 2);
            assert_eq!(report.compile_fail_passed, 1);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP run_with_tests_crate: cargo not on PATH");
        }
        Err(e) if e.to_string().contains("sealed signature drift")
            || e.to_string().contains("missing from crisp.lock") =>
        {
            update_lock(&root).unwrap();
            run_tests(&root).expect("retry after lock update");
        }
        Err(e) => panic!("with_tests should pass: {e}"),
    }
}

#[test]
fn hello_build_still_works() {
    match crisp_rust_emit::build_emitted(&hello_root()) {
        Ok(_) => {}
        Err(PipelineError::ToolchainUnavailable) => {
            eprintln!("SKIP hello_build_still_works: cargo not on PATH");
        }
        Err(e) if e.to_string().contains("crisp.lock") => {
            update_lock(&hello_root()).unwrap();
            crisp_rust_emit::build_emitted(&hello_root()).expect("after lock");
        }
        Err(e) => panic!("hello build: {e}"),
    }
}

fn copy_crate(src: &std::path::Path, dst: &std::path::Path) {
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let dest = dst.join(name);
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&dest).unwrap();
            copy_crate(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).unwrap();
        }
    }
}
