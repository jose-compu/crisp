use crisp_errors::{ErrorPass, format_errors_crate};
use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn fallible_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fallible")
}

#[test]
fn errors_fallible_snapshot() {
    let root = fallible_root();
    let typed = TypeChecker::check_crate(&root).expect("typecheck");
    let errors = ErrorPass::analyze_crate(&root).expect("errors");
    let out = format_errors_crate(&errors, &typed);
    insta::assert_snapshot!(out);
}
