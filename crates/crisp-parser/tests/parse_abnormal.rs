use crisp_parser::Parser;

#[test]
fn parse_rejects_unclosed_function_body() {
    let src = "f() = { x := 1";
    let mut p = Parser::new(src).expect("parser");
    assert!(p.parse_module().is_err());
}

#[test]
fn parse_rejects_trailing_garbage() {
    let src = "pub main() = 1 }}}";
    let mut p = Parser::new(src).expect("parser");
    assert!(p.parse_module().is_err());
}

#[test]
fn parse_rejects_unclosed_string() {
    let src = r#"pub main() = print("unclosed)"#;
    assert!(Parser::new(src).is_err());
}
