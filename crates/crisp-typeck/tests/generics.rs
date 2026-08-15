//! User-facing generics + parametric shapes (#70 / #71).

use crisp_typeck::{TypeChecker, TypeError, format_sig};
use std::path::PathBuf;

fn generics_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/generics")
}

fn shapes_generic_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/shapes_generic")
}

fn generics_implicit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/generics_implicit")
}

fn generics_pub_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/generics_pub")
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn sig_named<'a>(typed: &'a crisp_typeck::TypedCrate, name: &str) -> &'a crisp_typeck::InferredSig {
    typed
        .signatures
        .values()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("missing signature {name}"))
}

#[test]
fn generics_example_typechecks() {
    let typed = TypeChecker::check_crate(&generics_root()).expect("typecheck generics");
    let id = format_sig(sig_named(&typed, "id"));
    let first = format_sig(sig_named(&typed, "first"));
    let second = format_sig(sig_named(&typed, "second"));
    let unwrap_int = format_sig(sig_named(&typed, "unwrap_int"));
    let unwrap_str = format_sig(sig_named(&typed, "unwrap_str"));
    eprintln!("id: {id}");
    eprintln!("first: {first}");
    eprintln!("second: {second}");
    eprintln!("unwrap_int: {unwrap_int}");
    eprintln!("unwrap_str: {unwrap_str}");
    assert_eq!(id, "id<T: Clone>(x: T) -> T");
    assert_eq!(first, "first<A: Clone, B: Clone>(p: Pair<A, B>) -> A");
    assert_eq!(second, "second<A: Clone, B: Clone>(p: Pair<A, B>) -> B");
    assert_eq!(unwrap_int, "unwrap_int(b: Boxy<int>) -> int");
    assert_eq!(unwrap_str, "unwrap_str(b: Boxy<str>) -> str");
    let mut trait_unwraps: Vec<_> = typed
        .signatures
        .values()
        .filter(|s| s.name == "unwrap")
        .map(|s| (s.impl_ty.clone(), format_sig(s)))
        .collect();
    trait_unwraps.sort();
    assert!(
        trait_unwraps.iter().any(
            |(ty, sig)| ty.as_deref() == Some("IntBox") && sig == "unwrap(self: IntBox) -> int"
        ),
        "expected IntBox unwrap, got {trait_unwraps:?}"
    );
    assert!(
        trait_unwraps.iter().any(
            |(ty, sig)| ty.as_deref() == Some("StrBox") && sig == "unwrap(self: StrBox) -> str"
        ),
        "expected StrBox unwrap, got {trait_unwraps:?}"
    );
}

#[test]
fn shapes_generic_example_typechecks() {
    let typed = TypeChecker::check_crate(&shapes_generic_root()).expect("typecheck shapes_generic");
    assert_eq!(
        format_sig(sig_named(&typed, "unwrap_int")),
        "unwrap_int(b: Boxy<int>) -> int"
    );
    assert_eq!(
        format_sig(sig_named(&typed, "unwrap_str")),
        "unwrap_str(b: Boxy<str>) -> str"
    );
}

#[test]
fn parametric_shape_rejects_wrong_field_type() {
    let err = TypeChecker::check_crate(&fixture("shape_generic_mismatch"))
        .expect_err("StrBox must not satisfy Boxy<int>");
    let msg = err.to_string();
    eprintln!("mismatch: {msg}");
    assert!(
        matches!(err, TypeError::Unify(_)) || msg.contains("shape") || msg.contains("mismatch"),
        "{msg}"
    );
}

#[test]
fn free_type_names_typecheck_like_explicit_binders() {
    let typed =
        TypeChecker::check_crate(&generics_implicit_root()).expect("typecheck generics_implicit");
    assert_eq!(
        format_sig(sig_named(&typed, "id")),
        "id<T: Clone>(x: T) -> T"
    );
    assert_eq!(
        format_sig(sig_named(&typed, "first")),
        "first<A: Clone, B: Clone>(p: Pair<A, B>) -> A"
    );
    assert_eq!(
        format_sig(sig_named(&typed, "unwrap_int")),
        "unwrap_int(b: Boxy<int>) -> int"
    );
    assert!(
        typed
            .impl_trait_args
            .values()
            .any(|args| matches!(args.first(), Some(crisp_typeck::Ty::Int))),
        "expected inferred Wrapper<int>, got {:?}",
        typed.impl_trait_args
    );
    assert!(
        typed
            .impl_trait_args
            .values()
            .any(|args| matches!(args.first(), Some(crisp_typeck::Ty::Str))),
        "expected inferred Wrapper<str>, got {:?}",
        typed.impl_trait_args
    );
}

#[test]
fn explicit_binder_shadowing_type_is_error() {
    let err = TypeChecker::check_crate(&fixture("generic_shadows_type"))
        .expect_err("<T> must not shadow type T");
    let msg = err.to_string();
    eprintln!("shadow: {msg}");
    assert!(msg.contains("E0049") || msg.contains("shadow"), "{msg}");
}

#[test]
fn in_scope_type_is_not_a_parameter() {
    let typed = TypeChecker::check_crate(&fixture("in_scope_type_not_param"))
        .expect("T in scope is the struct");
    assert_eq!(format_sig(sig_named(&typed, "id")), "id(x: T) -> T");
    let err = TypeChecker::check_crate(&fixture("in_scope_type_rejects_int"))
        .expect_err("id(1) must not instantiate in-scope type T");
    let msg = err.to_string();
    eprintln!("in-scope T vs int: {msg}");
    assert!(
        matches!(err, TypeError::Unify(_)) || msg.contains("mismatch"),
        "{msg}"
    );
}

#[test]
fn rigid_param_rejects_concrete_return() {
    let err = TypeChecker::check_crate(&fixture("generic_return_mismatch"))
        .expect_err("id<T>(x: T) -> int must not typecheck");
    let msg = err.to_string();
    eprintln!("rigid return: {msg}");
    assert!(
        matches!(err, TypeError::Unify(_)) || msg.contains("mismatch") || msg.contains("T"),
        "{msg}"
    );
}

#[test]
fn publication_generalizes_unannotated_and_specializes_internal() {
    let typed = TypeChecker::check_crate(&generics_pub_root()).expect("typecheck generics_pub");
    let id = sig_named(&typed, "id");
    let once = sig_named(&typed, "once");
    let identity = sig_named(&typed, "identity");
    eprintln!("id: {}", format_sig(id));
    eprintln!("once: {}", format_sig(once));
    eprintln!("identity: {}", format_sig(identity));
    assert_eq!(format_sig(id), "id<T: Clone>(x: T) -> T");
    assert!(
        id.instantiations.iter().any(|s| s.contains("int"))
            && id.instantiations.iter().any(|s| s.contains("str")),
        "id instantiations: {:?}",
        id.instantiations
    );
    assert!(id.mono_args.is_none(), "id used at two types stays generic");
    assert_eq!(format_sig(once), "once<T: Clone>(x: T) -> T");
    assert!(
        once.mono_args.is_some(),
        "once used only at int is marked for mono emit"
    );
    assert_eq!(format_sig(identity), "identity<T: Clone>(x: T) -> T");
    assert!(identity.is_pub);
    assert!(
        identity.mono_args.is_none(),
        "pub identity is never specialized"
    );
}

#[test]
fn mut_binding_is_not_generalized() {
    let err = TypeChecker::check_crate(&fixture("mut_not_generalized"))
        .expect_err("mut f := id must pin after first use");
    let msg = err.to_string();
    eprintln!("value restriction: {msg}");
    assert!(
        matches!(err, TypeError::Unify(_)) || msg.contains("mismatch"),
        "{msg}"
    );
}
