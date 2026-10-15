//! Vec literals `[1.0, 2.0]` (#119).

use crisp_ast::expr::ExprKind;
use crisp_ast::item::Item;
use crisp_parser::Parser;

#[test]
fn parse_float_vec_literal() {
    let mut p = Parser::new("f() = [1.0, 2.0]").unwrap();
    let ast = p.parse_file().unwrap();
    let Item::Function(f) = &ast.items[0] else {
        panic!("expected function");
    };
    match &f.body.kind {
        ExprKind::Array(elems) => assert_eq!(elems.len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn parse_index_and_index_assign() {
    let mut p = Parser::new("f() = xs[0]").unwrap();
    let ast = p.parse_file().unwrap();
    let Item::Function(f) = &ast.items[0] else {
        panic!("expected function");
    };
    assert!(
        matches!(&f.body.kind, ExprKind::Index { .. }),
        "{:?}",
        f.body.kind
    );

    let mut p = Parser::new("g() = { xs[0] = 1.0 }").unwrap();
    let ast = p.parse_file().unwrap();
    let Item::Function(g) = &ast.items[0] else {
        panic!("expected function");
    };
    let ExprKind::Block(b) = &g.body.kind else {
        panic!("{:?}", g.body.kind);
    };
    let expr = match b.stmts.first() {
        Some(crisp_ast::expr::Stmt::Expr(e)) => e,
        _ => b.tail.as_ref().map(|t| t.as_ref()).expect("index assign"),
    };
    assert!(
        matches!(&expr.kind, ExprKind::IndexAssign { .. }),
        "{:?}",
        expr.kind
    );
}

#[test]
fn parse_range_slice() {
    let mut p = Parser::new("f() = xs[1..4]").unwrap();
    let ast = p.parse_file().unwrap();
    let Item::Function(f) = &ast.items[0] else {
        panic!("expected function");
    };
    match &f.body.kind {
        ExprKind::Slice { .. } => {}
        other => panic!("expected Slice, got {other:?}"),
    }
}

#[test]
fn parse_nested_vec_type() {
    let mut p = Parser::new("f(xs: vec<vec<float>>) = xs").unwrap();
    let ast = p.parse_file().unwrap();
    let Item::Function(f) = &ast.items[0] else {
        panic!("expected function");
    };
    let ty = f.params[0].ty.as_ref().expect("param type");
    assert!(
        matches!(ty.kind, crisp_ast::ty::TypeKind::Generic { .. }),
        "{:?}",
        ty.kind
    );
}

#[test]
fn parse_dyn_type() {
    let mut p = Parser::new("f(x: dyn Show) = x").unwrap();
    let ast = p.parse_file().unwrap();
    let Item::Function(f) = &ast.items[0] else {
        panic!("expected function");
    };
    let ty = f.params[0].ty.as_ref().expect("param type");
    assert!(
        matches!(ty.kind, crisp_ast::ty::TypeKind::Dyn(_)),
        "{:?}",
        ty.kind
    );
}
