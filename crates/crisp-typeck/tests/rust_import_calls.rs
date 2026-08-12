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
