//! Parser coverage for closures and implicit-closure sugar (#72, #87–#89).

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
fn parse_lambda_literal() {
    let e = parse_fn_body("|x| x + 1");
    assert!(matches!(e.kind, ExprKind::Lambda { .. }), "{:?}", e.kind);
}

#[test]
fn parse_empty_lambda() {
    let e = parse_fn_body("|| 1");
    let ExprKind::Lambda { params, .. } = e.kind else {
        panic!("{:?}", e.kind);
    };
    assert!(params.is_empty());
}

#[test]
fn parse_trailing_lambda() {
    let e = parse_fn_body("run { |x| x * 2 }");
    let ExprKind::Call { args, .. } = e.kind else {
        panic!("{:?}", e.kind);
    };
    assert_eq!(args.len(), 1);
    assert!(matches!(args[0].kind, ExprKind::Lambda { .. }));
}

#[test]
fn parse_trailing_lambda_after_args() {
    let e = parse_fn_body("map_int(xs) { |x| x * 2 }");
    let ExprKind::Call { args, .. } = e.kind else {
        panic!("{:?}", e.kind);
    };
    assert_eq!(args.len(), 2);
    assert!(matches!(args[1].kind, ExprKind::Lambda { .. }));
}

#[test]
fn parse_field_section() {
    let e = parse_fn_body(".name");
    let ExprKind::Lambda { params, body, .. } = e.kind else {
        panic!("{:?}", e.kind);
    };
    assert_eq!(params.len(), 1);
    assert!(matches!(body.kind, ExprKind::Field { .. }));
}

#[test]
fn parse_parenthesized_lambda() {
    let e = parse_fn_body("(|x| x + 1)");
    assert!(matches!(e.kind, ExprKind::Lambda { .. }), "{:?}", e.kind);
}
