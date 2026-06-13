use crisp_ownership::{OwnershipPass, format_ownership_crate};
use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn hello_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
}

#[test]
fn ownership_hello_snapshot() {
    let root = hello_root();
    let typed = TypeChecker::check_crate(&root).unwrap();
    let ownership = OwnershipPass::analyze_crate(&root).unwrap();
    let out = format_ownership_crate(&ownership, &typed);
    insta::assert_snapshot!(out);
}
