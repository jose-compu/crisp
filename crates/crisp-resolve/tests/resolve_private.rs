use crisp_resolve::{ResolveError, Resolver};
use std::fs;
#[test]
fn rejects_private_import() {
    let dir = std::env::temp_dir().join("crisp-resolve-private-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("crisp.toml"),
        r#"
[package]
name = "priv_test"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(
        dir.join("src/secret.crp"),
        r#"
helper() = 1
"#,
    )
    .unwrap();
    fs::write(
        dir.join("src/main.crp"),
        r#"
use secret { helper }
pub main() = helper()
"#,
    )
    .unwrap();

    let err = Resolver::resolve_crate(&dir).unwrap_err();
    assert!(matches!(
        err,
        ResolveError::PrivateImport { name, .. } if name == "helper"
    ));

    let _ = fs::remove_dir_all(&dir);
}
