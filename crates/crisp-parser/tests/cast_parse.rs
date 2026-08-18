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

#[test]
fn issue_140_as_float_plus_is_binary() {
    let e = parse_fn_body("n as float + 1.0");
    match e.kind {
        ExprKind::Binary {
            op: crisp_ast::expr::BinaryOp::Add,
            left,
            right,
        } => {
            assert!(
                matches!(left.kind, ExprKind::Cast { .. }),
                "left should be Cast, got {:?}",
                left.kind
            );
            assert!(
                matches!(right.kind, ExprKind::Float(_)),
                "right should be 1.0, got {:?}",
                right.kind
            );
        }
        other => panic!("expected Binary(Cast, Add, 1.0), got {other:?}"),
    }
}

#[test]
fn issue_140_trait_bounds_still_parse_in_annotations() {
    let src = "g(x: Show + Eq) = x\n";
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("Show + Eq annotation");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected Function");
    };
    let ty = f.params[0].ty.as_ref().expect("param type");
    assert!(
        matches!(ty.kind, crisp_ast::ty::TypeKind::Constrained { .. }),
        "expected Constrained, got {:?}",
        ty.kind
    );
}
