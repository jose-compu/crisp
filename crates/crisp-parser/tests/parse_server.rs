use crisp_parser::Parser;
use std::fs;
use std::path::PathBuf;

fn server_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/server/src/main.crp");
    fs::read_to_string(path).expect("read server example")
}

#[test]
fn parse_server_example() {
    let src = server_source();
    let mut parser = Parser::new(&src).expect("lexer");
    let file = parser.parse_file().expect("parse server");
    insta::assert_debug_snapshot!(file);
}
