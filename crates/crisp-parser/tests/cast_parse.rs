//! Postfix `as float` / `as int` (#112).

use crisp_ast::expr::ExprKind;
use crisp_ast::item::Item;
use crisp_parser::Parser;

fn parse_fn_body(src: &str) -> crisp_ast::expr::Expr {
    let file = format!("f() = {src}");
    let mut p = Parser::new(&file).unwrap_or_else(|e| panic!("parser: {e}"));
    let ast = p.parse_file().unwrap_or_else(|e| panic!("parse: {e}"));
    let Item::Function(f) = &ast.items[0] else {
        panic!("expected function");
    };
    f.body.clone()
}

#[test]
fn parse_as_float() {
    let e = parse_fn_body("n as float");
    assert!(matches!(e.kind, ExprKind::Cast { .. }), "{:?}", e.kind);
}

#[test]
fn parse_as_int() {
    let e = parse_fn_body("x as int");
    let ExprKind::Cast { ty, .. } = e.kind else {
        panic!("{:?}", e.kind);
    };
    match ty.kind {
        crisp_ast::ty::TypeKind::Named(id) => assert_eq!(id.name, "int"),
        other => panic!("{other:?}"),
    }
}
