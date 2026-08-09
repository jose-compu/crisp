//! Diagnostic formatting tests (spec §17.4).

use crisp_ast::Span;
use crisp_diagnostics::{
    Severity, format_ownership_contradiction, format_type_mismatch, format_unresolved_name,
};

#[test]
fn formatted_error_has_caret_and_note() {
    let src = "fn f() = g(x)\n";
    let span = Span::new(10, 11);
    let diag = format_ownership_contradiction(src, "x", "own", "&", span);
    eprintln!("{}", diag.rendered);
    assert!(diag.rendered.contains("ERROR [E0050]"));
    assert!(diag.rendered.contains("^"));
    assert!(diag.rendered.contains("note:"));
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn type_mismatch_diagnostic() {
    let src = "let x := 1\n";
    let diag = format_type_mismatch(src, "str", "int", Span::new(9, 10));
    eprintln!("{}", diag.rendered);
    assert!(diag.rendered.contains("E0041"));
    assert!(diag.rendered.contains("expected `str`"));
}

#[test]
fn unresolved_name_snapshot_with_snippet_and_help() {
    let src = "pub main() = {\n    log(missing_fn())\n}\n";
    // highlight `missing_fn` on line 2
    let start = src.find("missing_fn").expect("needle") as u32;
    let span = Span::new(start, start + "missing_fn".len() as u32);
    let diag = format_unresolved_name(
        "src/main.crp",
        src,
        "missing_fn",
        span,
        Some("`missing_fn` is defined in module `util`; add `use util { missing_fn }`"),
    );
    insta::assert_snapshot!(diag.rendered);
}
