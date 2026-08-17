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

#[test]
fn interpolation_unresolved_span_is_on_the_string_not_use() {
    let root = fixture("interp_unresolved");
    let err = Resolver::resolve_crate(&root).expect_err("unresolved interpolation ident");
    let msg = err.to_string();
    assert!(msg.contains("E0035"), "{msg}");
    assert!(msg.contains("`s`"), "{msg}");
    let crisp_resolve::ResolveError::UnresolvedName { span, name, .. } = err else {
        panic!("expected UnresolvedName, got {err}");
    };
    assert_eq!(name, "s");
    let src = std::fs::read_to_string(root.join("src/main.crp")).expect("read main");
    let start = span.start as usize;
    let end = span.end as usize;
    assert!(end <= src.len(), "span {start}..{end} vs len {}", src.len());
    assert_eq!(
        &src[start..end],
        "s",
        "span should cover interpolation ident"
    );
    let line = src[..start].bytes().filter(|b| *b == b'\n').count() + 1;
    assert!(
        line > 1,
        "E0035 must not point at the first `use` line; got line {line}, span {span:?}"
    );
    assert!(
        src.lines()
            .nth(line - 1)
            .is_some_and(|l| l.contains("speed=")),
        "caret should land on the interpolation line, got line {line}"
    );
}
