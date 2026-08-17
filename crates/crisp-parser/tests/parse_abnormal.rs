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

#[test]
fn issue_109_trailing_comma_error_is_on_line_two() {
    let src = "pub main() = {\n    foo(a, b,)\n}\n";
    let mut p = Parser::new(src).expect("lex");
    let err = p.parse_file().expect_err("trailing comma in call");
    let pos = err.byte_pos() as usize;
    let line = src[..pos].bytes().filter(|b| *b == b'\n').count() + 1;
    assert_eq!(line, 2, "pos={pos} err={err}");
    assert!(
        !err.primary_message().contains("at byte"),
        "{}",
        err.primary_message()
    );
}
