use crisp_reveal::{reveal_expand, reveal_map, reveal_seal, reveal_traits};
use std::path::PathBuf;

fn hello_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
}

#[test]
fn reveal_traits_hello() {
    let out = reveal_traits(&hello_root()).unwrap();
    assert!(out.contains("no shape traits") || out.contains("shape"));
}

#[test]
fn reveal_seal_hello() {
    let out = reveal_seal(&hello_root()).unwrap();
    assert!(out.contains("main::main"));
}

#[test]
fn reveal_expand_hello() {
    let out = reveal_expand(&hello_root()).unwrap();
    assert!(out.contains("greet"));
    assert!(out.contains("main"));
}

#[test]
fn reveal_map_hello() {
    let out = reveal_map(&hello_root()).unwrap();
    assert!(out.contains("greet"));
    assert!(out.contains("drop"));
}
