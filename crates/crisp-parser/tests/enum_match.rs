use crisp_ast::expr::ExprKind;
use crisp_ast::item::Item;
use crisp_parser::Parser;

#[test]
fn parse_qualified_enum_patterns() {
    let src = r#"
type Color = | Red | Custom(int, int)
f(c) = match c {
    Color.Red -> 1
    Color.Custom(r, g) -> r + g
}
"#;
    let file = Parser::new(src).unwrap().parse_file().expect("parse");
    assert!(!file.items.is_empty());
}

#[test]
fn parse_match_bare_ident_scrutinee() {
    // Must not treat `c {` as a struct literal.
    let src = r#"
f(c) = match c {
    x -> x
}
"#;
    let file = Parser::new(src).unwrap().parse_file().expect("parse");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected function");
    };
    assert!(
        matches!(f.body.kind, ExprKind::Match { .. }),
        "got {:?}",
        f.body.kind
    );
}
