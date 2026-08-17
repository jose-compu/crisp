//! Record / shape field separators (#111): commas optional; newlines still valid.

use crisp_ast::expr::ExprKind;
use crisp_ast::item::{Item, TypeBody};
use crisp_parser::Parser;

fn parse(src: &str) -> crisp_ast::item::SourceFile {
    let mut p = Parser::new(src).unwrap_or_else(|e| panic!("lex: {e}"));
    p.parse_file().unwrap_or_else(|e| panic!("parse: {e}"))
}

#[test]
fn struct_lit_comma_separated_fields() {
    let ast = parse(
        r#"
type Point = { x: float, y: float }
f() = Point { x: 1.0, y: 2.0 }
"#,
    );
    let Item::TypeDef(td) = &ast.items[0] else {
        panic!("type: {:?}", ast.items[0]);
    };
    let TypeBody::Struct(fields) = &td.body else {
        panic!("struct body");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name.name, "x");
    assert_eq!(fields[1].name.name, "y");

    let Item::Function(f) = &ast.items[1] else {
        panic!("fn");
    };
    let ExprKind::StructLit { fields, .. } = &f.body.kind else {
        panic!("lit: {:?}", f.body.kind);
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name.name, "x");
    assert_eq!(fields[1].name.name, "y");
}

#[test]
fn struct_lit_trailing_comma() {
    let ast = parse("f() = Point { x: 1.0, y: 2.0, }");
    let Item::Function(f) = &ast.items[0] else {
        panic!("fn");
    };
    let ExprKind::StructLit { fields, .. } = &f.body.kind else {
        panic!("{:?}", f.body.kind);
    };
    assert_eq!(fields.len(), 2);
}

#[test]
fn struct_lit_newlines_without_commas_still_parse() {
    let ast = parse(
        r#"
f() = Point {
    x: 1.0
    y: 2.0
}
"#,
    );
    let Item::Function(f) = &ast.items[0] else {
        panic!("fn");
    };
    let ExprKind::StructLit { fields, .. } = &f.body.kind else {
        panic!("{:?}", f.body.kind);
    };
    assert_eq!(fields.len(), 2);
}

#[test]
fn shape_comma_separated_fields() {
    let ast = parse("shape HasPos = { x: float, y: float }");
    let Item::ShapeDef(s) = &ast.items[0] else {
        panic!("shape");
    };
    assert_eq!(s.fields.len(), 2);
}
