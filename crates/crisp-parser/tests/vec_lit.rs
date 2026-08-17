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
