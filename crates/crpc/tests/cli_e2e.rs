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
    eprintln!("$ {bin} {} (status={})", args.join(" "), output.status);
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
        "hello",
        "server",
        "fallible",
        "math",
        "float_demo",
        "defaults",
        "sealed",
        "with_tests",
        "match",
        "enums",
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
        "nested_math",
        "vec2_methods",
        "point_impl",
        "feature_gallery",
        "rust_import",
        "rust_shadow",
    ] {
        run_ok(
            "crpc",
            &["check", &examples_dir().join(ex).to_string_lossy()],
        );
    }
}

#[test]
fn crpc_run_nested_math() {
    let out = run_ok(
        "crpc",
        &["run", &examples_dir().join("nested_math").to_string_lossy()],
    );
    assert!(out.contains("sum=3"), "output: {out}");
}

#[test]
fn crpc_run_vec2_methods() {
    let out = run_ok(
        "crpc",
        &[
            "run",
            &examples_dir().join("vec2_methods").to_string_lossy(),
        ],
    );
    assert!(out.contains("m=5"), "output: {out}");
}

#[test]
fn crpc_test_feature_gallery() {
    run_ok(
        "crpc",
        &[
            "test",
            &examples_dir().join("feature_gallery").to_string_lossy(),
        ],
    );
}

#[test]
fn crpc_run_rust_import() {
    let out = run_ok(
        "crpc",
        &["run", &examples_dir().join("rust_import").to_string_lossy()],
    );
    assert!(
        out.contains('1') || out.contains("true") || out.contains("\"n\""),
        "expected JSON round-trip output, got: {out}"
    );
}

#[test]
fn crpc_check_rust_shadow_prints_w0048() {
    let path = examples_dir()
        .join("rust_shadow")
        .to_string_lossy()
        .to_string();
    let output = Command::new(bin("crpc"))
        .args(["check", &path])
        .output()
        .expect("run crpc check");
    eprintln!(
        "$ crpc check rust_shadow (status={})\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "check failed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("W0048"),
        "expected W0048 on stderr, got: {stderr}"
    );
}

#[test]
fn crpc_run_rust_shadow() {
    let out = run_ok(
        "crpc",
        &["run", &examples_dir().join("rust_shadow").to_string_lossy()],
    );
    assert!(out.contains("crisp-config:shadow-ok"), "output: {out}");
}

#[test]
fn crpc_check_missing_use_prints_snippet_and_help() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crisp-resolve/tests/fixtures/missing_use");
    let output = Command::new(bin_path("crpc"))
        .args(["check", &fixture.to_string_lossy()])
        .output()
        .expect("spawn crpc");
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("stderr:\n{stderr}");
    assert!(!output.status.success());
    assert!(
        stderr.contains("ERROR [E0035]") || stderr.contains("[E0035]"),
        "{stderr}"
    );
    assert!(stderr.contains("helper"), "{stderr}");
    assert!(
        stderr.contains("help:") || stderr.contains("use util"),
        "{stderr}"
    );
}

#[test]
fn crpc_check_shape_def_ok() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crisp-resolve/tests/fixtures/shape_def");
    let output = Command::new(bin_path("crpc"))
        .args(["check", &fixture.to_string_lossy()])
        .output()
        .expect("spawn crpc");
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("stderr:\n{stderr}");
    assert!(
        output.status.success(),
        "shape_def should check under #61: {stderr}"
    );
}

#[test]
fn crpc_emit_hello() {
    run_ok(
        "crpc",
        &["emit", &examples_dir().join("hello").to_string_lossy()],
    );
    assert!(examples_dir().join("hello/target/rust/Cargo.toml").exists());
}

#[test]
fn crpc_build_and_run_hello() {
    let hello = examples_dir().join("hello").to_string_lossy().to_string();
    run_ok("crpc", &["build", &hello]);
    let out = run_ok("crpc", &["run", &hello]);
    assert!(out.contains("hello world"), "output: {out}");
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
        &[
            "ownership",
            &examples_dir().join("server").to_string_lossy(),
        ],
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
    let root = examples_dir()
        .join("patterns")
        .to_string_lossy()
        .to_string();
    run_ok("crpc", &["build", &root]);
    let out = run_ok("crpc", &["run", &root]);
    assert!(
        out.contains("small") || out.contains("tagged"),
        "output: {out}"
    );
}

#[test]
fn crpc_test_patterns() {
    run_ok(
        "crpc",
        &["test", &examples_dir().join("patterns").to_string_lossy()],
    );
}

#[test]
fn crpc_build_and_run_kitchen_sink() {
    let root = examples_dir()
        .join("kitchen_sink")
        .to_string_lossy()
        .to_string();
    run_ok("crpc", &["build", &root]);
    let out = run_ok("crpc", &["run", &root]);
    assert!(out.contains("port=3000"), "output: {out}");
}

#[test]
fn reveal_types_kitchen_sink() {
    let out = run_ok(
        "reveal",
        &[
            "types",
            &examples_dir().join("kitchen_sink").to_string_lossy(),
        ],
    );
    assert!(out.contains("parse_port") || out.contains("load_port"));
    assert!(out.contains("main"));
}

#[test]
fn reveal_ownership_ownership_demo() {
    let out = run_ok(
        "reveal",
        &[
            "ownership",
            &examples_dir().join("ownership_demo").to_string_lossy(),
        ],
    );
    assert!(out.contains("make_greeting"));
    assert!(out.contains("main"));
}

#[test]
fn reveal_help_lists_section_16_commands() {
    let out = run_ok("reveal", &["--help"]);
    for cmd in [
        "types",
        "ownership",
        "lifetimes",
        "errors",
        "traits",
        "rust",
        "seal",
        "expand",
        "diff",
        "map",
    ] {
        assert!(out.contains(cmd), "reveal --help missing `{cmd}`:\n{out}");
    }
    assert!(
        out.contains("§16") || out.contains("spec"),
        "expected §16/spec mention in help:\n{out}"
    );
}

#[test]
fn reveal_types_help_mentions_path() {
    let out = run_ok("reveal", &["types", "--help"]);
    assert!(out.contains("PATH") || out.contains("path"), "help:\n{out}");
}

#[test]
fn reveal_lifetimes_hello() {
    let out = run_ok(
        "reveal",
        &["lifetimes", &examples_dir().join("hello").to_string_lossy()],
    );
    assert!(
        out.contains("greet") || out.contains("main") || out.contains("'"),
        "lifetimes output: {out}"
    );
}

#[test]
fn reveal_diff_and_map_hello_smoke() {
    let hello = examples_dir().join("hello").to_string_lossy().to_string();
    let diff = run_ok("reveal", &["diff", &hello]);
    assert!(!diff.trim().is_empty(), "diff should print something");
    let map = run_ok("reveal", &["map", &hello]);
    assert!(!map.trim().is_empty(), "map should print something");
}

#[test]
fn reveal_bad_path_prints_hint() {
    let output = Command::new(bin_path("reveal"))
        .args(["types", "/tmp/crisp-reveal-no-such-crate-xyz"])
        .output()
        .expect("spawn reveal");
    assert!(!output.status.success());
    let err = String::from_utf8_lossy(&output.stderr);
    eprintln!("stderr:\n{err}");
    assert!(
        err.contains("hint:") || err.contains("crisp.toml") || err.contains("failed"),
        "stderr: {err}"
    );
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
