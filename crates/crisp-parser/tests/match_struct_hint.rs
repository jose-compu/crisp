//! Match / struct-literal parse help (#22).

use crisp_parser::Parser;

#[test]
fn match_arm_missing_arrow_mentions_struct_lit_parens() {
    // After `match color {`, an arm that looks like a struct pattern without `->`
    // should surface the struct-literal scrutinee help.
    let src = r#"
pub main() = {
    match color {
        Red
    }
}
"#;
    let mut p = Parser::new(src).expect("lex");
    let err = p.parse_file().expect_err("expected parse failure");
    let msg = err.to_string();
    assert!(msg.contains("help:"), "missing help: {msg}");
    assert!(
        msg.contains("struct literal") || msg.contains("parentheses"),
        "unexpected help text: {msg}"
    );
}

#[test]
fn bare_match_ident_scrutinee_still_parses() {
    let src = r#"
pub main() = {
    match color {
        Red -> 1
        _ -> 0
    }
}
"#;
    let mut p = Parser::new(src).expect("lex");
    p.parse_file().expect("match name { … } must parse");
}

#[test]
fn parenthesized_struct_scrutinee_parses() {
    let src = r#"
pub type Point = { x: int }

pub main() = {
    match (Point { x: 1 }) {
        _ -> 0
    }
}
"#;
    let mut p = Parser::new(src).expect("lex");
    p.parse_file()
        .expect("match (Struct { … }) { … } must parse");
}
