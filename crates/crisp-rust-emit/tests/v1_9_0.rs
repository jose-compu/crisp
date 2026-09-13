//! v1.9.0: extern rust vec FFI (#153), OS prelude (#151), lib crate root (#152).

use crisp_cir::CirBuilder;
use crisp_rust_emit::{PipelineError, emit_crate, emit_to_target, run_emitted, run_tests};
use std::fs;
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn issue_153_emit_vec_as_slice() {
    let cir = CirBuilder::build_crate(&example("path_dep")).expect("cir #153");
    let out = emit_crate(&cir);
    eprintln!("#153 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains(".as_slice()"),
        "vec FFI should emit .as_slice():\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("local_core::sum_f64"),
        "missing sum_f64:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("local_core::ones"),
        "missing ones:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("local_core::plot_heatmap"),
        "missing plot_heatmap:\n{}",
        out.lib_rs
    );
}

#[test]
fn issue_153_runtime_vec_ffi() {
    match run_tests(&example("path_dep")) {
        Ok(r) => {
            eprintln!(
                "#153 runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert!(
                r.runtime_passed >= 4,
                "expected path_dep vec FFI tests, got {}",
                r.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_153 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_153 harness: {e}"),
    }
}

#[test]
fn issue_151_os_scalars_emit_and_test() {
    let cir = CirBuilder::build_crate(&example("os_scalars")).expect("cir #151");
    let out = emit_crate(&cir);
    eprintln!("#151 lib.rs:\n{}", out.lib_rs);
    assert!(
        out.lib_rs.contains("std::env::var"),
        "env_or should use std::env::var:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("std::path::Path::new"),
        "path helpers:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("std::fs::write"),
        "fs_write:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("std::fs::create_dir_all"),
        "create_dir_all:\n{}",
        out.lib_rs
    );
    match run_tests(&example("os_scalars")) {
        Ok(r) => {
            eprintln!("#151 runtime={}", r.runtime_passed);
            assert!(
                r.runtime_passed >= 4,
                "expected os_scalars tests, got {}",
                r.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_151 run_tests: cargo not on PATH");
        }
        Err(e) => panic!("issue_151 harness: {e}"),
    }
}

#[test]
fn issue_152_lib_root_emits_lib_not_bin() {
    let root = example("lib_root");
    let out = emit_to_target(&root).expect("emit #152");
    let cargo = fs::read_to_string(out.out_dir.join("Cargo.toml")).expect("Cargo.toml");
    eprintln!("#152 Cargo.toml:\n{cargo}");
    assert!(cargo.contains("[lib]"), "expected [lib]:\n{cargo}");
    assert!(
        cargo.contains("path = \"src/lib.rs\""),
        "lib path:\n{cargo}"
    );
    assert!(
        !cargo.contains("[[bin]]"),
        "lib-only must not emit dummy [[bin]]:\n{cargo}"
    );
    assert!(out.out_dir.join("src/lib.rs").is_file());
    assert!(
        !out.out_dir.join("src/main.rs").is_file(),
        "must not write dummy main.rs"
    );
    let lib_rs = fs::read_to_string(out.out_dir.join("src/lib.rs")).unwrap();
    assert!(lib_rs.contains("pub fn hello"), "{lib_rs}");
}

#[test]
fn issue_152_lib_root_tests_and_no_run() {
    match run_tests(&example("lib_root")) {
        Ok(r) => {
            eprintln!("#152 runtime={}", r.runtime_passed);
            assert!(
                r.runtime_passed >= 1,
                "expected lib_root test, got {}",
                r.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_152 run_tests: cargo not on PATH");
            return;
        }
        Err(e) => panic!("issue_152 harness: {e}"),
    }
    match run_emitted(&example("lib_root")) {
        Err(PipelineError::NoBin) => {}
        Err(e) if e.to_string().contains("cargo not on PATH") => {}
        other => panic!("lib-only crisp run should be NoBin, got {other:?}"),
    }
}

#[test]
fn issue_152_consumer_calls_emitted_lib() {
    let lib_root = example("lib_root");
    let emitted = emit_to_target(&lib_root).expect("emit lib_root");
    let tmp = tempfile::tempdir().unwrap();
    let consumer = tmp.path();
    fs::create_dir_all(consumer.join("src")).unwrap();
    let lib_path = emitted.out_dir.canonicalize().unwrap();
    fs::write(
        consumer.join("crisp.toml"),
        format!(
            r#"
[package]
name = "lib_consumer"
version = "0.1.0"
edition = "2026"
[dependencies]
lib_root = {{ rust = true, path = "{}" }}
"#,
            lib_path.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    fs::write(
        consumer.join("src/lib_root.crpi"),
        "extern rust lib_root {\n    hello(unused: str) -> str\n}\n",
    )
    .unwrap();
    fs::write(
        consumer.join("src/main.crp"),
        r#"
use lib_root { hello }

pub main() = {
    print(hello("x"))
}

test "calls emitted lib" = {
    assert_eq(hello("x"), "ok")
}
"#,
    )
    .unwrap();
    match run_tests(consumer) {
        Ok(r) => {
            eprintln!("#152 consumer runtime={}", r.runtime_passed);
            assert!(
                r.runtime_passed >= 1,
                "consumer tests: {}",
                r.runtime_passed
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP issue_152 consumer: cargo not on PATH");
        }
        Err(e) => panic!("issue_152 consumer: {e}"),
    }
}
