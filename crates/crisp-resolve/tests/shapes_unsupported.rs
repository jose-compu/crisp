use crisp_resolve::{ResolveError, Resolver};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn shape_def_is_rejected_with_e0039() {
    let err = Resolver::resolve_crate(&fixture("shape_def")).expect_err("shape def unsupported");
    let msg = err.to_string();
    assert!(
        msg.contains("E0039") && msg.contains("shapes are not yet supported"),
        "unexpected: {msg}"
    );
    assert!(matches!(err, ResolveError::ShapesUnsupported { .. }));
}

#[test]
fn shape_bound_is_rejected_with_e0039() {
    let err =
        Resolver::resolve_crate(&fixture("shape_bound")).expect_err("shape bound unsupported");
    let msg = err.to_string();
    assert!(
        msg.contains("E0039") && msg.contains("HasPosition"),
        "unexpected: {msg}"
    );
}

#[test]
fn shape_used_as_named_type_is_rejected_with_e0039() {
    let err =
        Resolver::resolve_crate(&fixture("shape_as_type")).expect_err("shape-as-type unsupported");
    let msg = err.to_string();
    assert!(
        msg.contains("E0039") && msg.contains("HasPosition"),
        "unexpected: {msg}"
    );
}

#[test]
fn hello_example_still_resolves_without_shape_false_positive() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
    Resolver::resolve_crate(&root).expect("hello must still resolve");
}
