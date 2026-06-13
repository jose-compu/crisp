use crisp_reveal::reveal_errors;
use std::path::PathBuf;

#[test]
fn reveal_fallible_errors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fallible");
    let out = reveal_errors(&root).expect("reveal errors");
    insta::assert_snapshot!(out);
}

#[test]
fn reveal_hello_no_errors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
    let out = reveal_errors(&root).expect("reveal hello errors");
    assert!(!out.contains('!'));
}
