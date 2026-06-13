//! CLI end-to-end tests for `crpc` and `reveal` binaries.

use std::path::PathBuf;
use std::process::Command;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn bin(name: &str) -> PathBuf {
    std::env::var_os(format!("CARGO_BIN_EXE_{name}"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../target/debug")
                .join(name)
        })
}

fn run_ok(bin: &str, args: &[&str]) -> String {
    let output = Command::new(bin_path(bin))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {bin} {args:?}: {e}"));
    eprintln!(
        "$ {bin} {} (status={})",
        args.join(" "),
        output.status
    );
    if !output.stderr.is_empty() {
        eprintln!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    }
    if !output.stdout.is_empty() {
        eprintln!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    }
    assert!(output.status.success(), "{bin} {:?} failed", args);
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn run_fail(bin: &str, args: &[&str]) {
    let output = Command::new(bin_path(bin))
        .args(args)
        .output()
        .expect("spawn");
    assert!(!output.status.success(), "{bin} {:?} should fail", args);
}

fn bin_path(name: &str) -> PathBuf {
    bin(name)
}

#[test]
fn crpc_check_all_examples() {
    for ex in [
        "hello", "server", "fallible", "math", "defaults", "sealed", "with_tests",
        "match", "async_hello", "ffi", "stdlib_smoke", "patterns", "kitchen_sink", "ownership_demo", "inventory",
        "vec_ops", "fallible_chain", "async_spawn", "workshop", "unsafe_math", "data_pipeline",
    ] {
        run_ok("crpc", &["check", &examples_dir().join(ex).to_string_lossy()]);
    }
}

#[test]
fn crpc_emit_hello() {
    run_ok("crpc", &["emit", &examples_dir().join("hello").to_string_lossy()]);
    assert!(examples_dir().join("hello/target/rust/Cargo.toml").exists());
}

#[test]
fn crpc_build_and_run_hello() {
    let hello = examples_dir().join("hello").to_string_lossy().to_string();
    run_ok("crpc", &["build", &hello]);
    let out = run_ok("crpc", &["run", &hello]);
    assert!(out.contains("hello crisp"), "output: {out}");
}

#[test]
fn crpc_test_math() {
    run_ok(
        "crpc",
        &["test", &examples_dir().join("math").to_string_lossy()],
    );
}

#[test]
fn crpc_test_with_tests() {
    run_ok(
        "crpc",
        &["test", &examples_dir().join("with_tests").to_string_lossy()],
    );
}

#[test]
fn reveal_types_hello() {
    let out = run_ok(
        "reveal",
        &["types", &examples_dir().join("hello").to_string_lossy()],
    );
    assert!(out.contains("greet"));
    assert!(out.contains("main"));
}

#[test]
fn reveal_ownership_server() {
    let out = run_ok(
        "reveal",
        &["ownership", &examples_dir().join("server").to_string_lossy()],
    );
    assert!(out.contains("main"));
}

#[test]
fn reveal_errors_fallible() {
    let out = run_ok(
        "reveal",
        &["errors", &examples_dir().join("fallible").to_string_lossy()],
    );
    assert!(out.contains("CrispError") || out.contains("IoError"));
}

#[test]
fn reveal_seal_sealed_example() {
    let out = run_ok(
        "reveal",
        &["seal", &examples_dir().join("sealed").to_string_lossy()],
    );
    assert!(out.contains("main::main"));
    assert!(out.contains("main::greet"));
}

#[test]
fn reveal_rust_math() {
    let out = run_ok(
        "reveal",
        &["rust", &examples_dir().join("math").to_string_lossy()],
    );
    assert!(out.contains("mod arith"));
    assert!(out.contains("arith::sum"));
}

#[test]
fn reveal_expand_defaults() {
    let out = run_ok(
        "reveal",
        &["expand", &examples_dir().join("defaults").to_string_lossy()],
    );
    assert!(out.contains("main"));
}

#[test]
fn crpc_build_and_run_patterns() {
    let root = examples_dir().join("patterns").to_string_lossy().to_string();
    run_ok("crpc", &["build", &root]);
    let out = run_ok("crpc", &["run", &root]);
    assert!(out.contains("small") || out.contains("tagged"), "output: {out}");
}

#[test]
fn crpc_test_patterns() {
    run_ok("crpc", &["test", &examples_dir().join("patterns").to_string_lossy()]);
}

#[test]
fn crpc_build_and_run_kitchen_sink() {
    let root = examples_dir().join("kitchen_sink").to_string_lossy().to_string();
    run_ok("crpc", &["build", &root]);
    let out = run_ok("crpc", &["run", &root]);
    assert!(out.contains("port=3000"), "output: {out}");
}

#[test]
fn reveal_types_kitchen_sink() {
    let out = run_ok(
        "reveal",
        &["types", &examples_dir().join("kitchen_sink").to_string_lossy()],
    );
    assert!(out.contains("parse_port") || out.contains("load_port"));
    assert!(out.contains("main"));
}

#[test]
fn reveal_ownership_ownership_demo() {
    let out = run_ok(
        "reveal",
        &["ownership", &examples_dir().join("ownership_demo").to_string_lossy()],
    );
    assert!(out.contains("make_greeting"));
    assert!(out.contains("main"));
}

#[test]
fn sealed_drift_fails_check() {
    let dir = tempfile::TempDir::new().unwrap();
    let src = examples_dir().join("sealed");
    copy_dir(&src, dir.path());
    let lock_path = dir.path().join("crisp.lock");
    let mut raw = std::fs::read_to_string(&lock_path).unwrap();
    raw = raw.replace("greet(name: &str)", "greet(name: &str) DRIFT");
    std::fs::write(&lock_path, raw).unwrap();
    run_fail("crpc", &["check", &dir.path().to_string_lossy()]);
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), to).unwrap();
        }
    }
}
