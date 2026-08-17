//! Loop / if-assign typeck (#96).

use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn float_bisection_if_assign_typecks() {
    let typed = TypeChecker::check_crate(&fixture("bisection_width"))
        .expect("while + if-then assign should typeck (#96)");
    assert!(
        typed
            .signatures
            .values()
            .any(|s| s.name == "bisection_width"),
        "missing bisection_width: {:?}",
        typed.signatures.keys().collect::<Vec<_>>()
    );
}
