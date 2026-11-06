//! Explicit `use` of a prelude name binds the imported item (#173).

use crisp_cir::CirBuilder;
use crisp_rust_emit::{emit_crate, run_emitted, run_tests};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn prelude_shadow_emits_crate_shim_not_std_path() {
    let cir = CirBuilder::build_crate(&example("prelude_shadow")).expect("cir #173");
    let out = emit_crate(&cir);
    eprintln!("#173 lib.rs:\n{}", out.lib_rs);
    for (name, body) in &out.modules {
        eprintln!("#173 {name}.rs:\n{body}");
    }
    let combined = format!(
        "{}\n{}",
        out.lib_rs,
        out.modules
            .iter()
            .map(|(_, b)| b.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        combined.contains("crate::shim::path_is_file"),
        "import should emit crate::shim::path_is_file:\n{combined}"
    );
    assert!(
        !combined.contains("Path::is_file"),
        "must not lower to prelude Path::is_file:\n{combined}"
    );
}

#[test]
fn prelude_shadow_runtime_is_always_true() {
    match run_emitted(&example("prelude_shadow")) {
        Ok(out) => {
            eprintln!("run stdout: {out:?}");
            assert!(
                out.contains("true") || out.contains("1"),
                "shim returns true, stdout={out:?}"
            );
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP: cargo not on PATH");
        }
        Err(e) => panic!("prelude_shadow run: {e}"),
    }
}

#[test]
fn prelude_shadow_tests_pass() {
    match run_tests(&example("prelude_shadow")) {
        Ok(r) => {
            eprintln!(
                "runtime={} compile_fail={}",
                r.runtime_passed, r.compile_fail_passed
            );
            assert!(r.runtime_passed > 0);
        }
        Err(e) if e.to_string().contains("cargo not on PATH") => {
            eprintln!("SKIP: cargo not on PATH");
        }
        Err(e) => panic!("prelude_shadow tests: {e}"),
    }
}
