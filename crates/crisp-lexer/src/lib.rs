//! Lexical analysis for `.crp` sources (spec §2).

mod lexer;
mod token;

pub use lexer::{LexError, Lexer};
pub use token::{Kw, Token, TokenKind};

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).tokenize()
}

/// Rust keywords that are legal Crisp identifiers (spec §2.3).
pub const RUST_KEYWORDS: &[&str] = &[
    "fn", "let", "impl", "move", "dyn", "async", "await", "struct", "enum", "trait", "type",
    "where", "for", "loop", "match", "if", "else", "return", "break", "continue", "const",
    "static", "mut", "ref", "self", "Self", "super", "crate", "pub", "use", "mod", "unsafe",
    "extern", "true", "false", "as", "in", "box", "yield", "try", "macro", "union",
];

pub fn is_rust_keyword(name: &str) -> bool {
    RUST_KEYWORDS.contains(&name)
}
