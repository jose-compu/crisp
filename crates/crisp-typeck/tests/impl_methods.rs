//! Inherent `impl Type` methods (§5.4 / #20).

use crisp_typeck::{TypeChecker, TypeError, format_sig};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
}

#[test]
fn point_impl_typechecks_and_registers_methods() {
    let typed = TypeChecker::check_crate(&example("point_impl")).expect("point_impl");
    assert!(
        typed.inherent_methods.get("Point").is_some_and(|m| {
            m.contains_key("new") && m.contains_key("translate") && m.contains_key("manhattan")
        }),
        "expected Point inherent methods, got {:?}",
        typed.inherent_methods
    );
    assert!(typed.signatures.contains_key("main::Point::new"));
    assert!(typed.signatures.contains_key("main::Point::manhattan"));
}

#[test]
fn vec2_methods_nested_typechecks() {
    let typed = TypeChecker::check_crate(&example("vec2_methods")).expect("vec2_methods");
    assert!(
        typed
            .inherent_methods
            .get("Vec2")
            .is_some_and(|m| m.contains_key("magnitude")),
        "Vec2 methods: {:?}",
        typed.inherent_methods
    );
    assert!(
        typed
            .signatures
            .contains_key("math.vector::Vec2::magnitude")
    );
}

#[test]
fn feature_gallery_typechecks() {
    TypeChecker::check_crate(&example("feature_gallery")).expect("feature_gallery");
}

#[test]
fn show_trait_typechecks_and_registers_methods() {
    let typed = TypeChecker::check_crate(&example("show_trait")).expect("show_trait");
    assert!(
        typed
            .inherent_methods
            .get("Point")
            .is_some_and(|m| m.contains_key("show")),
        "expected Point.show from trait impl, got {:?}",
        typed.inherent_methods
    );
    assert!(typed.signatures.contains_key("main::Point::show"));
    let label = typed
        .signatures
        .values()
        .find(|s| s.name == "label")
        .expect("label");
    assert_eq!(format_sig(label), "label<T: Clone + Show>(x: T) -> str");
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn issue_59_dyn_is_e0091() {
    let err = TypeChecker::check_crate(&fixture("dyn_trait")).expect_err("E0091");
    let msg = err.to_string();
    eprintln!("dyn: {msg}");
    assert!(
        matches!(err, TypeError::UnsupportedDyn { .. }) || msg.contains("E0091"),
        "{msg}"
    );
}

#[test]
fn issue_59_extra_impl_method_is_e0092() {
    let err = TypeChecker::check_crate(&fixture("trait_extra_method")).expect_err("E0092 extra");
    let msg = err.to_string();
    eprintln!("extra: {msg}");
    assert!(
        matches!(err, TypeError::TraitImpl { .. }) || msg.contains("E0092"),
        "{msg}"
    );
    assert!(msg.contains("extra"), "{msg}");
}

#[test]
fn issue_59_missing_impl_method_is_e0092() {
    let err =
        TypeChecker::check_crate(&fixture("trait_missing_method")).expect_err("E0092 missing");
    let msg = err.to_string();
    eprintln!("missing: {msg}");
    assert!(
        matches!(err, TypeError::TraitImpl { .. }) || msg.contains("E0092"),
        "{msg}"
    );
    assert!(msg.contains("show"), "{msg}");
}
