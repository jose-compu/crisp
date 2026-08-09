use crisp_resolve::Resolver;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn unresolved_name_hints_defining_module() {
    let err = Resolver::resolve_crate(&fixture("missing_use")).expect_err("missing use");
    let msg = err.to_string();
    assert!(msg.contains("E0035"), "{msg}");
    assert!(msg.contains("help:"), "{msg}");
    assert!(msg.contains("util"), "{msg}");
    assert!(msg.contains("use util"), "{msg}");
}
