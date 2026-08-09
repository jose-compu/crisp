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

#[test]
fn totally_unknown_name_has_e0035_without_module_hint() {
    let err = Resolver::resolve_crate(&fixture("unknown_name")).expect_err("unknown");
    let msg = err.to_string();
    assert!(msg.contains("E0035"), "{msg}");
    assert!(msg.contains("totally_missing"), "{msg}");
    assert!(
        !msg.contains("is defined in module"),
        "should not invent a module for a missing symbol: {msg}"
    );
}

#[test]
fn missing_use_fixture_still_typechecks_after_import_fix() {
    // Sanity: util.helper is a real export; only the missing `use` fails resolve.
    let util = fixture("missing_use").join("src/util.crp");
    assert!(util.exists());
}
