//! v1.11.0: JSON literals (#166/#169), `json` in extern rust (#168), `[T; N]` (#167).

use crisp_typeck::{TypeChecker, TypeError, format_sig, is_json_ty};
use std::fs;
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
}

fn write_crate(dir: &std::path::Path, main: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("crisp.toml"),
        r#"
[package]
name = "t"
version = "0.1.0"
edition = "2026"
[dependencies]
serde_json = { rust = true, version = "1" }
"#,
    )
    .unwrap();
    fs::write(dir.join("src/main.crp"), main).unwrap();
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
fn issue_166_json_literals_example_typechecks() {
    let typed = TypeChecker::check_crate(&example("json_literals")).expect("json_literals");
    let nested = typed
        .signatures
        .values()
        .find(|s| s.name == "nested_array")
        .expect("nested_array");
    eprintln!("nested_array: {}", format_sig(nested));
    assert!(is_json_ty(&nested.ret), "{}", format_sig(nested));
}

#[test]
fn issue_167_fixed_array_example_typechecks() {
    let typed = TypeChecker::check_crate(&example("fixed_arrays")).expect("fixed_arrays");
    let triple = typed
        .signatures
        .values()
        .find(|s| s.name == "triple")
        .expect("triple");
    eprintln!("triple: {}", format_sig(triple));
    assert_eq!(format_sig(triple), "triple() -> [float; 3]");
}

#[test]
fn issue_167_wrong_len_is_e0094() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(
        tmp.path(),
        r#"
take2(xs: [float; 2]) = xs[0]
pub main() = { print(take2([1.0, 2.0, 3.0])) }
"#,
    );
    let err = TypeChecker::check_crate(tmp.path()).expect_err("len");
    let msg = err.to_string();
    eprintln!("len: {msg}");
    assert!(
        matches!(err, TypeError::ArrayLen { .. }) || msg.contains("E0094"),
        "{msg}"
    );
}

#[test]
fn issue_168_extern_json_allowed() {
    let tmp = tempfile::tempdir().unwrap();
    write_extern_fixture(
        tmp.path(),
        r#"
use kernels { take_json }

pub main() = {
    print(take_json({ a: 1 }))
}
"#,
        r#"
extern rust kernels {
    take_json(body: json) -> str
}
"#,
    );
    let typed = TypeChecker::check_crate(tmp.path()).expect("json extern #168");
    let take = typed
        .signatures
        .get("rust.kernels::take_json")
        .expect("take_json");
    assert!(is_json_ty(&take.params[0].1), "{take:?}");
}

#[test]
fn issue_168_map_still_e0090() {
    let tmp = tempfile::tempdir().unwrap();
    write_extern_fixture(
        tmp.path(),
        r#"
use kernels { bad }
pub main() = { bad() }
"#,
        r#"
extern rust kernels {
    bad(xs: map) -> int
}
"#,
    );
    let err = TypeChecker::check_crate(tmp.path()).expect_err("map");
    let msg = err.to_string();
    eprintln!("map: {msg}");
    assert!(msg.contains("E0090"), "{msg}");
}

#[test]
fn issue_169_json_array_ret() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(
        tmp.path(),
        r#"
use serde_json { to_string }
extern rust serde_json { to_string(v: json) -> str! }
nested() -> json = [1, true, "x"]
pub main() = { print(to_string(nested())) }
"#,
    );
    TypeChecker::check_crate(tmp.path()).expect("json array #169");
}
