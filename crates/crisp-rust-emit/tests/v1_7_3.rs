//! v1.7.3: path deps (#105) and `crisp run` cwd (#106).

use crisp_rust_emit::{PipelineError, emit_to_target, run_emitted};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
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
fn issue_105_path_dep_rewritten_in_emitted_cargo_toml() {
    let root = example("path_dep");
    match emit_to_target(&root) {
        Err(e) => skip_or_panic(e, "#105 emit"),
        Ok(out) => {
            let cargo =
                std::fs::read_to_string(out.out_dir.join("Cargo.toml")).expect("Cargo.toml");
            eprintln!("#105 Cargo.toml:\n{cargo}");
            assert!(
                cargo.contains("local_core") && cargo.contains("path = "),
                "expected path dep in emitted Cargo.toml:\n{cargo}"
            );
            assert!(
                cargo.contains("../../vendor/local_core"),
                "path should be relative to target/rust:\n{cargo}"
            );
            assert!(
                !cargo.contains("path = \"vendor/local_core\""),
                "unrewritten crate-root path would not resolve from target/rust:\n{cargo}"
            );
        }
    }
}

#[test]
fn issue_105_path_dep_example_runs() {
    match run_emitted(&example("path_dep")) {
        Err(e) => skip_or_panic(e, "#105 run"),
        Ok(out) => {
            eprintln!("#105 stdout: {out:?}");
            assert!(
                out.contains("42"),
                "expected local_core::answer() == 42, got {out:?}"
            );
        }
    }
}
