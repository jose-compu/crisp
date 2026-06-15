//! Diagnostic formatting tests (spec §17.4).

use crisp_ast::Span;
use crisp_diagnostics::{Severity, format_ownership_contradiction, format_type_mismatch};

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
