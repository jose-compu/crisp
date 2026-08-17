use crisp_reveal::{reveal_expand, reveal_map, reveal_seal, reveal_traits, reveal_types};
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

#[test]
fn reveal_types_shows_pub_scheme_and_clone() {
    let out = reveal_types(&example("generics_pub")).unwrap();
    eprintln!("reveal types generics_pub:\n{out}");
    assert!(
        out.contains("identity<T: Clone>(x: T) -> T"),
        "scheme: {out}"
    );
    assert!(out.contains("id<T: Clone>(x: T) -> T"), "id: {out}");
    assert!(
        out.contains("once<T: Clone>(x: T) -> T") && out.contains("used as int"),
        "once scheme + inst: {out}"
    );
}

#[test]
fn reveal_types_shows_int_float_coercions() {
    let out = reveal_types(&example("math")).unwrap();
    eprintln!("reveal types math:\n{out}");
    assert!(
        out.contains("coercion") && out.contains("as float"),
        "inserted int→float coercions must be visible in reveal types: {out}"
    );
}
