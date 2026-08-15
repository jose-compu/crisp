use crisp_ast::Item;
use crisp_parser::Parser;
use std::fs;
use std::path::PathBuf;

fn hello_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello/src/main.crp");
    fs::read_to_string(path).expect("read hello example")
}

#[test]
fn parse_hello_example() {
    let src = hello_source();
    let mut parser = Parser::new(&src).expect("lexer");
    let file = parser.parse_file().expect("parse hello");
    insta::assert_debug_snapshot!(file);
}

#[test]
fn parse_hello_has_main_and_greet() {
    let src = hello_source();
    let mut parser = Parser::new(&src).expect("lexer");
    let items = parser.parse_module().expect("parse hello");
    let fns: Vec<_> = items
        .iter()
        .filter_map(|item| match item {
            Item::Function(f) => Some(f.name.name.as_str()),
            _ => None,
        })
        .collect();
    assert!(fns.contains(&"greet"));
    assert!(fns.contains(&"main"));
}
