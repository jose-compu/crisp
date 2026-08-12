use crisp_resolve::{ResolveError, Resolver};
use std::fs;
use std::path::PathBuf;

fn write_fixture(dir: &std::path::Path, toml: &str, main: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("crisp.toml"), toml).unwrap();
    fs::write(dir.join("src/main.crp"), main).unwrap();
}

#[test]
fn resolves_use_rust_with_rust_true_dep() {
    let tmp = tempfile::tempdir().unwrap();
    write_fixture(
        tmp.path(),
        r#"
[package]
name = "rust_use"
version = "0.1.0"
edition = "2026"
[dependencies]
serde_json = { rust = true, version = "1" }
"#,
        r#"
use rust.serde_json { from_str }

main = {
  -- import only; typeck/emit of calls lands in a later PR
  ()
}
"#,
    );

    let resolved = Resolver::resolve_crate(tmp.path()).expect("resolve");
    assert!(
        resolved
            .rust_imports
            .iter()
            .any(|i| i.crate_name == "serde_json"
                && i.item == "from_str"
                && i.local_name == "from_str")
    );
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .unwrap();
    assert!(main.scope.iter().any(|s| s == "from_str"));
}

#[test]
fn resolves_use_rust_colon_colon_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    write_fixture(
        tmp.path(),
        r#"
[package]
name = "rust_use_cc"
version = "0.1.0"
edition = "2026"
[dependencies]
serde_json = { rust = true, version = "1" }
"#,
        "use rust::serde_json { from_str }\nmain = { () }\n",
    );
    let resolved = Resolver::resolve_crate(tmp.path()).expect("resolve");
    assert_eq!(resolved.rust_imports.len(), 1);
}

#[test]
fn rejects_missing_rust_dep() {
    let tmp = tempfile::tempdir().unwrap();
    write_fixture(
        tmp.path(),
        r#"
[package]
name = "no_dep"
version = "0.1.0"
edition = "2026"
"#,
        "use rust.serde_json { from_str }\nmain = { () }\n",
    );
    let err = Resolver::resolve_crate(tmp.path()).unwrap_err();
    assert!(
        matches!(err, ResolveError::RustCrateNotFound { .. }),
        "{err}"
    );
    assert!(err.to_string().contains("E0044"));
}

#[test]
fn rejects_dep_without_rust_true() {
    let tmp = tempfile::tempdir().unwrap();
    write_fixture(
        tmp.path(),
        r#"
[package]
name = "unmarked"
version = "0.1.0"
edition = "2026"
[dependencies]
serde_json = "1"
"#,
        "use rust.serde_json { from_str }\nmain = { () }\n",
    );
    let err = Resolver::resolve_crate(tmp.path()).unwrap_err();
    assert!(
        matches!(err, ResolveError::RustCrateNotMarked { .. }),
        "{err}"
    );
    assert!(err.to_string().contains("E0045"));
}

#[test]
fn rejects_bare_use_rust_without_list() {
    let tmp = tempfile::tempdir().unwrap();
    write_fixture(
        tmp.path(),
        r#"
[package]
name = "bare"
version = "0.1.0"
edition = "2026"
[dependencies]
serde_json = { rust = true, version = "1" }
"#,
        "use rust.serde_json\nmain = { () }\n",
    );
    let err = Resolver::resolve_crate(tmp.path()).unwrap_err();
    assert!(
        matches!(err, ResolveError::RustImportNeedsList { .. }),
        "{err}"
    );
}

#[test]
fn examples_ffi_still_resolves() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/ffi");
    Resolver::resolve_crate(&root).expect("ffi example");
}
