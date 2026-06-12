//! Lexical analysis for `.crp` sources (spec §2).

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    IntLit(i64),
    Ident(String),
    // TODO: keywords, operators (&&&, |||, ^^^), delimiters
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Error)]
pub enum LexError {
    #[error("invalid character at byte {0}")]
    InvalidChar(u32),
}

pub fn lex(_source: &str) -> Result<Vec<Token>, LexError> {
    Ok(vec![Token {
        kind: TokenKind::Eof,
        start: 0,
        end: 0,
    }])
}
