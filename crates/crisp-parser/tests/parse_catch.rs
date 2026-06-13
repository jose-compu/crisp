use crisp_parser::Parser;

#[test]
fn parse_catch_expression() {
    let src = r#"
f() = read_config("x") catch _ -> Config { port: 0 }
"#;
    let mut parser = Parser::new(src).expect("parser");
    let file = parser.parse_file().expect("parse");
    let crisp_ast::item::Item::Function(f) = &file.items[0] else {
        panic!("expected function");
    };
    assert!(matches!(
        f.body.kind,
        crisp_ast::expr::ExprKind::Catch { .. }
    ));
}
