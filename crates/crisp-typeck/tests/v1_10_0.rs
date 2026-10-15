//! v1.10.0: slices (#159), extern rust vec<str>/vec<bool> (#158), trait impl / dyn (#59).

use crisp_typeck::{TypeChecker, TypeError, format_sig};
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
fn issue_159_slice_type_is_vec() {
    let typed = TypeChecker::check_crate(&example("vec_ops")).expect("vec_ops #159");
    let mid = typed
        .signatures
        .values()
        .find(|s| s.name == "mid")
        .expect("mid");
    eprintln!("mid: {}", format_sig(mid));
    assert_eq!(format_sig(mid), "mid<T: Clone>(xs: vec<T>) -> vec<T>");
}

#[test]
fn issue_158_vec_str_and_bool_allowed() {
    let tmp = tempfile::tempdir().unwrap();
    write_extern_fixture(
        tmp.path(),
        r#"
use kernels { join_names, all_true }

pub main() = {
    print(join_names(["a", "b"]))
    print(all_true([true, false]))
}
"#,
        r#"
extern rust kernels {
    join_names(xs: vec<str>) -> str
    all_true(xs: vec<bool>) -> bool
}
"#,
    );
    let typed = TypeChecker::check_crate(tmp.path()).expect("vec<str>/vec<bool> #158");
    let join = typed
        .signatures
        .get("rust.kernels::join_names")
        .expect("join_names");
    assert!(matches!(join.ret, crisp_typeck::Ty::Str));
    let flags = typed
        .signatures
        .get("rust.kernels::all_true")
        .expect("all_true");
    assert!(matches!(flags.ret, crisp_typeck::Ty::Bool));
}

#[test]
fn issue_158_nested_vec_is_e0090() {
    let tmp = tempfile::tempdir().unwrap();
    write_extern_fixture(
        tmp.path(),
        r#"
use kernels { bad }

pub main() = {
    bad()
}
"#,
        r#"
extern rust kernels {
    bad(xs: vec<vec<float>>) -> float
}
"#,
    );
    let err = TypeChecker::check_crate(tmp.path()).expect_err("nested vec FFI");
    let msg = err.to_string();
    eprintln!("nested vec: {msg}");
    assert!(msg.contains("E0090"), "{msg}");
}

#[test]
fn issue_158_path_dep_typechecks() {
    TypeChecker::check_crate(&example("path_dep")).expect("path_dep #158");
}

#[test]
fn issue_59_dyn_is_e0091() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(
        tmp.path(),
        r#"
trait Show = {
    show(self) -> str
}

id(x: dyn Show) = x

pub main() = {
    print("no")
}
"#,
    );
    let err = TypeChecker::check_crate(tmp.path()).expect_err("dyn");
    let msg = err.to_string();
    eprintln!("dyn: {msg}");
    assert!(
        matches!(err, TypeError::UnsupportedDyn { .. }) || msg.contains("E0091"),
        "{msg}"
    );
}

#[test]
fn issue_59_extra_trait_method_is_e0092() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(
        tmp.path(),
        r#"
trait Show = {
    show(self) -> str
}

type Point = {
    x: int
}

impl Show for Point = {
    show(self) = "p"
    extra(self) = 1
}

pub main() = {
    print("no")
}
"#,
    );
    let err = TypeChecker::check_crate(tmp.path()).expect_err("extra method");
    let msg = err.to_string();
    eprintln!("extra: {msg}");
    assert!(
        matches!(err, TypeError::TraitImpl { .. }) || msg.contains("E0092"),
        "{msg}"
    );
    assert!(msg.contains("extra"), "{msg}");
}

#[test]
fn issue_59_missing_required_method_is_e0092() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(
        tmp.path(),
        r#"
trait Show = {
    show(self) -> str
}

type Point = {
    x: int
}

impl Show for Point = {}

pub main() = {
    print("no")
}
"#,
    );
    let err = TypeChecker::check_crate(tmp.path()).expect_err("missing method");
    let msg = err.to_string();
    eprintln!("missing: {msg}");
    assert!(
        matches!(err, TypeError::TraitImpl { .. }) || msg.contains("E0092"),
        "{msg}"
    );
    assert!(msg.contains("show"), "{msg}");
}

#[test]
fn issue_59_trait_defaults_still_typecheck() {
    TypeChecker::check_crate(&example("trait_defaults")).expect("trait_defaults");
    TypeChecker::check_crate(&example("show_trait")).expect("show_trait");
    TypeChecker::check_crate(&example("std_traits")).expect("std_traits");
}
