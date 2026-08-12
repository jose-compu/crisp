use crisp_resolve::Resolver;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn shape_def_resolves() {
    Resolver::resolve_crate(&fixture("shape_def")).expect("shape def should resolve (#61)");
}

#[test]
fn shape_bound_resolves() {
    Resolver::resolve_crate(&fixture("shape_bound")).expect("shape bound should resolve (#61)");
}

#[test]
fn shape_used_as_named_type_resolves() {
    Resolver::resolve_crate(&fixture("shape_as_type")).expect("shape-as-type should resolve (#61)");
}

#[test]
fn hello_example_still_resolves_without_shape_false_positive() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
    Resolver::resolve_crate(&root).expect("hello must still resolve");
}

#[test]
fn shapes_example_resolves() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/shapes");
    Resolver::resolve_crate(&root).expect("examples/shapes must resolve");
}
