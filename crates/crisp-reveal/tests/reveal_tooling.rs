use crisp_reveal::{reveal_expand, reveal_map, reveal_seal, reveal_traits};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
}

fn hello_root() -> PathBuf {
    example("hello")
}

#[test]
fn reveal_traits_hello() {
    let out = reveal_traits(&hello_root()).unwrap();
    assert!(
        out.contains("no traits in this crate") || out.contains("shape") || out.contains("trait "),
        "unexpected reveal traits output: {out}"
    );
}

#[test]
fn reveal_traits_show_trait() {
    let out = reveal_traits(&example("show_trait")).unwrap();
    assert!(out.contains("trait Show"), "output: {out}");
    assert!(out.contains("impl Show for Point"), "output: {out}");
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
