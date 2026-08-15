//! Resolve coverage for user generics and parametric shapes (#70 / #71).

use crisp_resolve::Resolver;
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
}

#[test]
fn generics_example_resolves() {
    Resolver::resolve_crate(&example("generics")).expect("examples/generics must resolve");
}

#[test]
fn shapes_generic_example_resolves() {
    Resolver::resolve_crate(&example("shapes_generic"))
        .expect("examples/shapes_generic must resolve");
}

#[test]
fn generics_implicit_example_resolves() {
    Resolver::resolve_crate(&example("generics_implicit"))
        .expect("examples/generics_implicit must resolve");
}
