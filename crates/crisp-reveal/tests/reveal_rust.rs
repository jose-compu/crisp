use crisp_reveal::reveal_rust;
use std::path::PathBuf;

#[test]
fn reveal_rust_hello_snapshot() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
    let out = reveal_rust(&root).expect("reveal rust");
    assert!(out.contains("fn main"));
    insta::assert_snapshot!("reveal_rust_hello", out);
}
