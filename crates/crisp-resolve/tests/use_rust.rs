use crisp_resolve::{ResolveError, ResolveWarning, Resolver};
use std::fs;
use std::path::PathBuf;

fn write_fixture(dir: &std::path::Path, toml: &str, main: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("crisp.toml"), toml).unwrap();
    fs::write(dir.join("src/main.crp"), main).unwrap();
}

fn write_collision_fixture(dir: &std::path::Path) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("crisp.toml"),
        r#"
[package]
name = "shadow"
version = "0.1.0"
edition = "2026"
[dependencies]
config = { rust = true, version = "0.1" }
"#,
    )
    .unwrap();
    fs::write(
        dir.join("src/config.crp"),
        r#"
pub report(msg: str) = {
  ()
}
"#,
    )
    .unwrap();
    fs::write(
        dir.join("src/main.crp"),
        r#"
use config { report }

main = {
  report("hi")
}
"#,
    )
    .unwrap();
}

#[test]
fn resolves_bare_use_crate_like_typescript() {
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
use serde_json { from_str }

main = {
  ()
}
"#,
    );

    let resolved = Resolver::resolve_crate(tmp.path()).expect("resolve");
    assert!(
        resolved
            .rust_imports
            .iter()
            .any(|i| i.crate_name == "serde_json" && i.item == "from_str")
    );
    assert!(resolved.warnings.is_empty());
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .unwrap();
    assert!(main.scope.iter().any(|s| s == "from_str"));
}

#[test]
fn resolves_use_rust_prefix_alias() {
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
fn collision_warns_and_binds_crisp_module() {
    let tmp = tempfile::tempdir().unwrap();
    write_collision_fixture(tmp.path());

    let resolved = Resolver::resolve_crate(tmp.path()).expect("resolve");
    assert!(
        resolved.warnings.iter().any(|w| matches!(
            w,
            ResolveWarning::ModuleShadowsRustDep { name, .. } if name == "config"
        )),
        "warnings: {:?}",
        resolved.warnings
    );
    assert!(
        resolved.warnings[0].to_string().contains("W0048"),
        "{}",
        resolved.warnings[0]
    );
    // Crisp module wins: no rust_imports for config.
    assert!(resolved.rust_imports.is_empty());
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .unwrap();
    assert!(main.scope.iter().any(|s| s == "report"));
    let report = main
        .imports
        .iter()
        .find(|b| b.local_name == "report")
        .expect("report binding");
    assert_eq!(report.symbol.module, "config");
}

#[test]
fn collision_rust_prefix_forces_crate() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(
        tmp.path().join("crisp.toml"),
        r#"
[package]
name = "shadow_force"
version = "0.1.0"
edition = "2026"
[dependencies]
config = { rust = true, version = "0.1" }
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("src/config.crp"),
        "pub report(msg: str) = { () }\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("src/main.crp"),
        "use rust.config { builder }\nmain = { () }\n",
    )
    .unwrap();

    let resolved = Resolver::resolve_crate(tmp.path()).expect("resolve");
    assert!(
        resolved
            .rust_imports
            .iter()
            .any(|i| i.crate_name == "config" && i.item == "builder")
    );
    // Prefix path does not emit the shadow warning (explicit disambiguation).
    assert!(resolved.warnings.is_empty());
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
fn rejects_bare_dep_without_rust_true() {
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
        "use serde_json { from_str }\nmain = { () }\n",
    );
    let err = Resolver::resolve_crate(tmp.path()).unwrap_err();
    assert!(
        matches!(err, ResolveError::RustCrateNotMarked { .. }),
        "{err}"
    );
    assert!(err.to_string().contains("E0045"));
}

#[test]
fn rejects_dep_without_rust_true_prefix() {
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
        "use serde_json\nmain = { () }\n",
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

#[test]
fn example_rust_import_resolves_bare_and_alias() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/rust_import");
    let resolved = Resolver::resolve_crate(&root).expect("rust_import");
    assert!(resolved.warnings.is_empty());
    assert!(
        resolved
            .rust_imports
            .iter()
            .any(|i| i.crate_name == "serde_json" && i.item == "from_str")
    );
    assert!(
        resolved
            .rust_imports
            .iter()
            .any(|i| i.crate_name == "serde_json" && i.item == "to_string")
    );
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .unwrap();
    assert!(main.scope.iter().any(|s| s == "from_str"));
    assert!(main.scope.iter().any(|s| s == "to_string"));
}

#[test]
fn example_rust_shadow_warns_w0048() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/rust_shadow");
    let resolved = Resolver::resolve_crate(&root).expect("rust_shadow");
    assert!(
        resolved.warnings.iter().any(|w| matches!(
            w,
            ResolveWarning::ModuleShadowsRustDep { name, .. } if name == "config"
        )),
        "expected W0048, got {:?}",
        resolved.warnings
    );
    assert!(resolved.rust_imports.is_empty());
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .unwrap();
    let report = main
        .imports
        .iter()
        .find(|b| b.local_name == "report")
        .expect("report");
    assert_eq!(report.symbol.module, "config");
}
