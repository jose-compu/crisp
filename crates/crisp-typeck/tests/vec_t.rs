//! Implicit `vec<T>` from `new`/`push` and `[ ]` literals (#119).

use crisp_typeck::{TypeChecker, TypeError, format_sig};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn issue_119_float_literal_and_push() {
    let typed = TypeChecker::check_crate(&fixture("vec_float")).expect("vec<T> #119");
    let floats = typed
        .signatures
        .values()
        .find(|s| s.name == "floats")
        .expect("floats");
    assert_eq!(format_sig(floats), "floats() -> vec<float>");
    let ints = typed
        .signatures
        .values()
        .find(|s| s.name == "ints")
        .expect("ints");
    assert_eq!(format_sig(ints), "ints() -> vec<int>");
    let empty_f = typed
        .signatures
        .values()
        .find(|s| s.name == "empty_f")
        .expect("empty_f");
    assert_eq!(format_sig(empty_f), "empty_f() -> vec<float>");
}

#[test]
fn issue_119_uninferred_new_is_e0088() {
    let err = TypeChecker::check_crate(&fixture("vec_uninferred")).expect_err("E0088");
    assert!(
        matches!(err, TypeError::UninferredVec { .. }) || err.to_string().contains("E0088"),
        "{err}"
    );
}
