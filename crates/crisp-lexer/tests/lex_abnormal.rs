use crisp_lexer::{TokenKind, lex};

#[test]
fn lex_nested_block_comments() {
    let src = "x := 1 {- a {- b -} c -} y := 2";
    let tokens = lex(src).expect("lex");
    let ids: Vec<_> = tokens
        .iter()
        .filter_map(|t| match &t.kind {
            TokenKind::Ident(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ids, vec!["x", "y"]);
}

#[test]
fn lex_rejects_invalid_escape_in_char() {
    let src = "'\\q'";
    let err = lex(src).expect_err("bad char escape");
    eprintln!("{err}");
}
