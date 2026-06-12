//! Recursive-descent / Pratt parser for Crisp.

use crisp_ast::item::Item;
use crisp_lexer::{lex, Token};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("lex error: {0}")]
    Lex(#[from] crisp_lexer::LexError),
    #[error("unexpected token at offset {0}")]
    Unexpected(u32),
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(source: &str) -> Result<Self, ParseError> {
        Ok(Self {
            tokens: lex(source)?,
            pos: 0,
        })
    }

    pub fn parse_module(&mut self) -> Result<Vec<Item>, ParseError> {
        Ok(vec![])
    }
}
