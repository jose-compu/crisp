use crisp_reveal::{reveal_lifetimes, reveal_ownership};
use std::path::PathBuf;

fn hello_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
}

#[test]
fn reveal_hello_ownership() {
    let out = reveal_ownership(&hello_root()).expect("reveal ownership");
    insta::assert_snapshot!(out);
}

#[test]
fn reveal_hello_lifetimes() {
    let out = reveal_lifetimes(&hello_root()).expect("reveal lifetimes");
    insta::assert_snapshot!(out);
}

#[test]
fn reveal_server_ownership() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/server");
    let out = reveal_ownership(&root).expect("reveal server ownership");
    insta::assert_snapshot!(out);
}
