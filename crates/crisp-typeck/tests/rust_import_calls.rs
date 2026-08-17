use crisp_typeck::TypeChecker;
use std::fs;

fn write_fixture(dir: &std::path::Path, main: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("crisp.toml"),
        r#"
[package]
name = "rust_calls"
version = "0.1.0"
edition = "2026"
[dependencies]
serde_json = { rust = true, version = "1" }
"#,
    )
    .unwrap();
    fs::write(dir.join("src/main.crp"), main).unwrap();
}

#[test]
fn typechecks_serde_json_from_str_to_string() {
    let tmp = tempfile::tempdir().unwrap();
    write_fixture(
        tmp.path(),
        r#"
use serde_json { from_str, to_string }

pub main() = {
    v := from_str("[1, true]")
    s := to_string(v)
    print(s)
}
"#,
    );
    let typed = TypeChecker::check_crate(tmp.path()).expect("typeck");
    assert!(
        typed
            .rust_imports
            .iter()
            .any(|i| i.item == "from_str" && i.crate_name == "serde_json")
    );
    assert!(typed.signatures.contains_key("rust.serde_json::from_str"));
    assert!(typed.signatures.contains_key("rust.serde_json::to_string"));
}

#[test]
fn example_rust_import_typechecks() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/rust_import");
    TypeChecker::check_crate(&root).expect("rust_import typeck");
}

fn write_extern_fixture(dir: &std::path::Path, main: &str, sidecar: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("crisp.toml"),
        r#"
[package]
name = "extern_calls"
version = "0.1.0"
edition = "2026"
[dependencies]
kernels = { rust = true, path = "vendor/kernels" }
"#,
    )
    .unwrap();
    fs::write(dir.join("src/main.crp"), main).unwrap();
    fs::write(dir.join("src/kernels.crpi"), sidecar).unwrap();
}

#[test]
fn extern_rust_sidecar_types_lap3() {
    let tmp = tempfile::tempdir().unwrap();
    write_extern_fixture(
        tmp.path(),
        r#"
use kernels { lap3 }

pub main() = {
    print(lap3(0.0, 1.0, 0.0, 1.0))
}
"#,
        r#"
extern rust kernels {
    lap3(um: float, uc: float, up: float, dx: float) -> float
}
"#,
    );
    let typed = TypeChecker::check_crate(tmp.path()).expect("typeck #116");
    let sig = typed
        .signatures
        .get("rust.kernels::lap3")
        .expect("lap3 sig");
    assert!(matches!(sig.ret, crisp_typeck::Ty::Float));
    assert_eq!(sig.params.len(), 4);
}

#[test]
fn undeclared_rust_import_is_e0089() {
    let tmp = tempfile::tempdir().unwrap();
    write_extern_fixture(
        tmp.path(),
        r#"
use kernels { mystery }

pub main() = {
    mystery(1.0)
}
"#,
        r#"
extern rust kernels {
    lap3(um: float, uc: float, up: float, dx: float) -> float
}
"#,
    );
    let err = TypeChecker::check_crate(tmp.path()).expect_err("undeclared");
    let msg = err.to_string();
    assert!(msg.contains("E0089"), "{msg}");
    assert!(msg.contains("mystery"), "{msg}");
}

#[test]
fn example_path_dep_typechecks_with_extern() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/path_dep");
    TypeChecker::check_crate(&root).expect("path_dep typeck");
}
