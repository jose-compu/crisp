//! Diagnostic formatting tests (spec §17.4 / #22).

use crisp_ast::Span;
use crisp_diagnostics::{
    Severity, format_diagnostic_at, format_ownership_contradiction, format_parse_error,
    format_type_mismatch, format_unresolved_name,
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

#[test]
fn unresolved_name_default_help_when_no_hint() {
    let src = "log(x)\n";
    let span = Span::new(4, 5);
    let diag = format_unresolved_name("src/main.crp", src, "x", span, None);
    assert!(diag.rendered.contains("= help:"));
    assert!(diag.rendered.contains("use"));
    assert!(diag.rendered.contains("--> src/main.crp:"));
}

#[test]
fn shape_unsupported_snippet_snapshot() {
    let src = "shape HasPosition = {\n    x: float\n}\n";
    let start = src.find("HasPosition").expect("needle") as u32;
    let span = Span::new(start, start + "HasPosition".len() as u32);
    let diag = format_diagnostic_at(
        "src/main.crp",
        src,
        "E0039",
        "shapes are not yet supported (`HasPosition`)",
        span,
        Severity::Error,
        &["help: remove the `shape` definition or bound".into()],
    );
    insta::assert_snapshot!(diag.rendered);
}

#[test]
fn ambiguous_field_snippet_includes_annotation_help() {
    let src = "sku_of(item) = item.sku\n";
    let start = src.find("sku").expect("needle") as u32;
    let span = Span::new(start, start + 3);
    let diag = format_diagnostic_at(
        "src/catalog.crp",
        src,
        "E0043",
        "ambiguous field `sku` on unresolved type; annotate the parameter (candidates: Item, StockLine)",
        span,
        Severity::Error,
        &["help: write `param: StructName` on the function parameter".into()],
    );
    assert!(diag.rendered.contains("ERROR [E0043]"));
    assert!(diag.rendered.contains("--> src/catalog.crp:"));
    assert!(diag.rendered.contains("= help:"));
    assert!(diag.rendered.contains("^"));
}

#[test]
fn parse_error_renders_file_line_col_and_byte_note() {
    let src = "pub main() = {\n    foo(a, b,)\n}\n";
    let pos = (src.find("b,)").expect("needle") + 2) as u32;
    let diag = format_parse_error(
        "src/main.crp",
        src,
        "E0010",
        "unexpected token `)`, expected identifier",
        pos,
        &[],
    );
    eprintln!("{}", diag.rendered);
    assert!(diag.rendered.contains("ERROR [E0010]"));
    assert!(
        diag.rendered.contains("--> src/main.crp:2:"),
        "want line 2, got {}",
        diag.rendered
    );
    assert!(diag.rendered.contains("^"));
    assert!(
        !diag.rendered.contains("at byte"),
        "byte must not be the primary location: {}",
        diag.rendered
    );
    assert!(diag.rendered.contains("byte offset"));
}
