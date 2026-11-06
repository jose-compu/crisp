//! Prelude names may be shadowed by an explicit `use` (#173).

use crisp_resolve::{ResolveError, Resolver};
use std::fs;
use std::path::PathBuf;

fn write_pkg(dir: &std::path::Path, toml: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("crisp.toml"), toml).unwrap();
}

#[test]
fn rust_import_of_path_is_file_shadows_prelude() {
    let tmp = tempfile::tempdir().unwrap();
    write_pkg(
        tmp.path(),
        r#"
[package]
name = "shim_clash"
version = "0.1.0"
edition = "2026"
[dependencies]
shim = { rust = true, version = "0.1" }
"#,
    );
    fs::write(
        tmp.path().join("src/main.crp"),
        r#"
use shim { path_is_file }

pub main() = {
    print(path_is_file("."))
}
"#,
    )
    .unwrap();

    let resolved = Resolver::resolve_crate(tmp.path()).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("rust_imports: {:?}", resolved.rust_imports);
    assert!(
        resolved
            .rust_imports
            .iter()
            .any(|i| i.crate_name == "shim" && i.item == "path_is_file"),
        "expected rust.shim path_is_file, got {:?}",
        resolved.rust_imports
    );
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .unwrap();
    let binding = main
        .imports
        .iter()
        .find(|b| b.local_name == "path_is_file")
        .expect("path_is_file binding");
    assert_eq!(binding.symbol.module, "rust.shim");
    assert_eq!(binding.symbol.name, "path_is_file");
}

#[test]
fn rust_import_of_path_join_and_path_parent_shadows_prelude() {
    let tmp = tempfile::tempdir().unwrap();
    write_pkg(
        tmp.path(),
        r#"
[package]
name = "shim_paths"
version = "0.1.0"
edition = "2026"
[dependencies]
lacan_shim = { rust = true, version = "0.1" }
"#,
    );
    fs::write(
        tmp.path().join("src/main.crp"),
        r#"
use lacan_shim { path_join, path_parent, cwd }

pub main() = {
    ()
}
"#,
    )
    .unwrap();

    let resolved = Resolver::resolve_crate(tmp.path()).unwrap_or_else(|e| panic!("{e}"));
    for item in ["path_join", "path_parent", "cwd"] {
        assert!(
            resolved
                .rust_imports
                .iter()
                .any(|i| i.crate_name == "lacan_shim" && i.item == item),
            "missing {item}: {:?}",
            resolved.rust_imports
        );
    }
}

#[test]
fn crisp_module_path_is_file_shadows_prelude() {
    let tmp = tempfile::tempdir().unwrap();
    write_pkg(
        tmp.path(),
        r#"
[package]
name = "mod_clash"
version = "0.1.0"
edition = "2026"
"#,
    );
    fs::write(
        tmp.path().join("src/shim.crp"),
        "pub path_is_file(path: str) -> bool = { true }\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("src/main.crp"),
        "use shim { path_is_file }\npub main() = { print(path_is_file(\".\")) }\n",
    )
    .unwrap();

    let resolved = Resolver::resolve_crate(tmp.path()).unwrap_or_else(|e| panic!("{e}"));
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .unwrap();
    let binding = main
        .imports
        .iter()
        .find(|b| b.local_name == "path_is_file")
        .expect("path_is_file binding");
    eprintln!("binding module={}", binding.symbol.module);
    assert_eq!(binding.symbol.module, "shim");
}

#[test]
fn alias_avoids_prelude_name() {
    let tmp = tempfile::tempdir().unwrap();
    write_pkg(
        tmp.path(),
        r#"
[package]
name = "alias_ok"
version = "0.1.0"
edition = "2026"
[dependencies]
shim = { rust = true, version = "0.1" }
"#,
    );
    fs::write(
        tmp.path().join("src/main.crp"),
        "use shim { path_is_file as is_file }\npub main() = { () }\n",
    )
    .unwrap();
    let resolved = Resolver::resolve_crate(tmp.path()).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        resolved
            .rust_imports
            .iter()
            .any(|i| i.local_name == "is_file" && i.item == "path_is_file")
    );
}

#[test]
fn two_module_imports_are_e0038_and_name_both() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ambiguous_import");
    let err = Resolver::resolve_crate(&root).expect_err("two foos");
    let msg = err.to_string();
    eprintln!("E0038: {msg}");
    assert!(msg.contains("E0038"), "{msg}");
    assert!(msg.contains("`foo`"), "{msg}");
    assert!(msg.contains("`a`"), "{msg}");
    assert!(msg.contains("`b`"), "{msg}");
    match err {
        ResolveError::AmbiguousImport {
            name,
            first,
            second,
            span,
        } => {
            assert_eq!(name, "foo");
            assert!(
                (first == "a" && second == "b") || (first == "b" && second == "a"),
                "modules {first} / {second}"
            );
            assert!(span.end > span.start, "use span should be non-empty");
        }
        other => panic!("expected AmbiguousImport, got {other}"),
    }
}

#[test]
fn example_prelude_shadow_resolves() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/prelude_shadow");
    let resolved = Resolver::resolve_crate(&root).expect("prelude_shadow");
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .unwrap();
    let binding = main
        .imports
        .iter()
        .find(|b| b.local_name == "path_is_file")
        .expect("path_is_file");
    assert_eq!(binding.symbol.module, "shim");
}
