#![allow(clippy::needless_return)]

use crate::token::{Kw, Token, TokenKind};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum LexError {
    #[error("unexpected character `{ch}`")]
    UnexpectedChar { ch: char, pos: u32 },
    #[error("unterminated block comment")]
    UnterminatedBlockComment { pos: u32 },
    #[error("unterminated string")]
    UnterminatedString { pos: u32 },
    #[error("unterminated char literal")]
    UnterminatedChar { pos: u32 },
    #[error("invalid escape in string")]
    InvalidEscape { pos: u32 },
    #[error("invalid number literal")]
    InvalidNumber { pos: u32 },
}

impl LexError {
    pub fn byte_pos(&self) -> u32 {
        match self {
            LexError::UnexpectedChar { pos, .. }
            | LexError::UnterminatedBlockComment { pos }
            | LexError::UnterminatedString { pos }
            | LexError::UnterminatedChar { pos }
            | LexError::InvalidEscape { pos }
            | LexError::InvalidNumber { pos } => *pos,
        }
    }
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<(u32, char)>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let chars = source.char_indices().map(|(i, c)| (i as u32, c)).collect();
        Self {
            source,
            chars,
            pos: 0,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments()?;
            let start = self.offset();
            if self.is_eof() {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    start,
                    end: start,
                });
                break;
            }
            let kind = self.scan_token()?;
            let end = self.offset();
            tokens.push(Token { kind, start, end });
        }
        Ok(tokens)
    }

    fn offset(&self) -> u32 {
        self.chars
            .get(self.pos)
            .map(|(i, _)| *i)
            .unwrap_or(self.source.len() as u32)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|(_, c)| *c)
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).map(|(_, c)| *c)
    }

    fn bump(&mut self) -> Option<char> {
        if self.is_eof() {
            return None;
        }
        let (_, c) = self.chars[self.pos];
        self.pos += 1;
        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
                self.bump();
            }
            if self.peek() == Some('-') && self.peek2() == Some('-') {
                self.bump();
                self.bump();
                while let Some(c) = self.peek() {
                    self.bump();
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            if self.peek() == Some('{') && self.peek2() == Some('-') {
                let start = self.offset();
                self.bump();
                self.bump();
                self.skip_block_comment(start)?;
                continue;
            }
            break;
        }
        Ok(())
    }

    fn skip_block_comment(&mut self, start: u32) -> Result<(), LexError> {
        let mut depth = 1usize;
        while !self.is_eof() {
            match (self.peek(), self.peek2()) {
                (Some('{'), Some('-')) => {
                    self.bump();
                    self.bump();
                    depth += 1;
                }
                (Some('-'), Some('}')) => {
                    self.bump();
                    self.bump();
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                _ => {
                    self.bump();
                }
            }
        }
        Err(LexError::UnterminatedBlockComment { pos: start })
    }

    fn scan_token(&mut self) -> Result<TokenKind, LexError> {
        let start = self.offset();
        let c = self.bump().unwrap();

        match c {
            '(' => return Ok(TokenKind::LParen),
            ')' => return Ok(TokenKind::RParen),
            '{' => return Ok(TokenKind::LBrace),
            '}' => return Ok(TokenKind::RBrace),
            '[' => return Ok(TokenKind::LBracket),
            ']' => return Ok(TokenKind::RBracket),
            ',' => return Ok(TokenKind::Comma),
            ';' => return Ok(TokenKind::Semi),
            ':' => {
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::ColonEq);
                }
                return Ok(TokenKind::Colon);
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Ok(TokenKind::DotDotEq);
                    }
                    return Ok(TokenKind::DotDot);
                }
                return Ok(TokenKind::Dot);
            }
            '+' => {
                if self.peek() == Some('+') {
                    self.bump();
                    return Ok(TokenKind::PlusPlus);
                }
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::PlusEq);
                }
                return Ok(TokenKind::Plus);
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.bump();
                    return Ok(TokenKind::Arrow);
                }
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::MinusEq);
                }
                return Ok(TokenKind::Minus);
            }
            '*' => {
                if self.peek() == Some('*') {
                    self.bump();
                    return Ok(TokenKind::StarStar);
                }
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::StarEq);
                }
                return Ok(TokenKind::Star);
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::SlashEq);
                }
                return Ok(TokenKind::Slash);
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::PercentEq);
                }
                return Ok(TokenKind::Percent);
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::EqEq);
                }
                return Ok(TokenKind::Assign);
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::Ne);
                }
                return Ok(TokenKind::Bang);
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::Le);
                }
                if self.peek() == Some('<') {
                    self.bump();
                    return Ok(TokenKind::Shl);
                }
                return Ok(TokenKind::Lt);
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.bump();
                    return Ok(TokenKind::Ge);
                }
                if self.peek() == Some('>') {
                    self.bump();
                    return Ok(TokenKind::Shr);
                }
                return Ok(TokenKind::Gt);
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.bump();
                    if self.peek() == Some('|') {
                        self.bump();
                        return Ok(TokenKind::PipePipePipe);
                    }
                    return Ok(TokenKind::Or);
                }
                if self.peek() == Some('>') {
                    self.bump();
                    return Ok(TokenKind::PipeGt);
                }
                return Ok(TokenKind::Pipe);
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.bump();
                    if self.peek() == Some('&') {
                        self.bump();
                        return Ok(TokenKind::AmpAmpAmp);
                    }
                    return Ok(TokenKind::And);
                }
                if self.peek() == Some('m') {
                    let rest = self.rest_from_pos();
                    if rest.starts_with("mut")
                        && !rest.chars().nth(3).is_some_and(is_ident_continue)
                    {
                        self.pos += 3;
                        return Ok(TokenKind::AmpMut);
                    }
                }
                return Ok(TokenKind::Amp);
            }
            '^' => {
                if self.peek() == Some('^') {
                    self.bump();
                    if self.peek() == Some('^') {
                        self.bump();
                        return Ok(TokenKind::CaretCaretCaret);
                    }
                }
                return Ok(TokenKind::Caret);
            }
            '~' => return Ok(TokenKind::Tilde),
            '?' => return Ok(TokenKind::Question),
            '\'' => {
                if self.peek().is_some_and(is_ident_start) {
                    return self.scan_lifetime();
                }
                return self.scan_char(start);
            }
            '"' => return self.scan_string(start, false),
            'r' if self.peek() == Some('"') => {
                self.bump();
                return self.scan_string(start, true);
            }
            _ if is_ident_start(c) => return self.scan_ident_or_keyword(c),
            _ if c.is_ascii_digit() => return self.scan_number(c, start),
            _ => Err(LexError::UnexpectedChar { ch: c, pos: start }),
        }
    }

    fn rest_from_pos(&self) -> &str {
        let byte = self.offset() as usize;
        &self.source[byte..]
    }

    fn scan_ident_or_keyword(&mut self, first: char) -> Result<TokenKind, LexError> {
        let start = self.offset() - first.len_utf8() as u32;
        let mut name = String::from(first);
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                name.push(self.bump().unwrap());
            } else {
                break;
            }
        }
        // mut:= is special
        if name == "mut" && self.peek() == Some(':') && self.peek2() == Some('=') {
            self.bump();
            self.bump();
            return Ok(TokenKind::MutColonEq);
        }
        if let Some(kw) = Kw::lookup(&name) {
            return Ok(TokenKind::Kw(kw));
        }
        let _ = start;
        Ok(TokenKind::Ident(name))
    }

    fn scan_lifetime(&mut self) -> Result<TokenKind, LexError> {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                name.push(self.bump().unwrap());
            } else {
                break;
            }
        }
        Ok(TokenKind::Lifetime(name))
    }

    fn scan_char(&mut self, start: u32) -> Result<TokenKind, LexError> {
        let ch = if self.peek() == Some('\\') {
            self.bump();
            self.scan_escape()?
        } else {
            self.bump()
                .ok_or(LexError::UnterminatedChar { pos: start })?
        };
        if self.bump() != Some('\'') {
            return Err(LexError::UnterminatedChar { pos: start });
        }
        Ok(TokenKind::Char(ch))
    }

    fn scan_string(&mut self, start: u32, raw: bool) -> Result<TokenKind, LexError> {
        // triple-quote?
        if !raw && self.peek() == Some('"') && self.peek2() == Some('"') {
            self.bump();
            self.bump();
            return self.scan_multiline_string(start);
        }

        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(LexError::UnterminatedString { pos: start }),
                Some('"') => {
                    self.bump();
                    break;
                }
                Some('\\') if !raw => {
                    self.bump();
                    let esc = self.scan_escape()?;
                    s.push(esc);
                }
                Some(_) => {
                    s.push(self.bump().unwrap());
                }
            }
        }
        Ok(TokenKind::String(s))
    }

    fn scan_multiline_string(&mut self, start: u32) -> Result<TokenKind, LexError> {
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(LexError::UnterminatedString { pos: start }),
                Some('"')
                    if self.peek2() == Some('"')
                        && self.chars.get(self.pos + 2).map(|(_, c)| *c) == Some('"') =>
                {
                    self.bump();
                    self.bump();
                    self.bump();
                    break;
                }
                Some(_) => {
                    s.push(self.bump().unwrap());
                }
            }
        }
        Ok(TokenKind::String(s))
    }

    fn scan_escape(&mut self) -> Result<char, LexError> {
        let pos = self.offset();
        let c = self.bump().ok_or(LexError::InvalidEscape { pos })?;
        match c {
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            '\\' => Ok('\\'),
            '"' => Ok('"'),
            '\'' => Ok('\''),
            'u' => {
                if self.bump() != Some('{') {
                    return Err(LexError::InvalidEscape { pos });
                }
                let mut hex = String::new();
                while let Some(ch) = self.peek() {
                    if ch == '}' {
                        self.bump();
                        break;
                    }
                    hex.push(self.bump().unwrap());
                }
                let code =
                    u32::from_str_radix(&hex, 16).map_err(|_| LexError::InvalidEscape { pos })?;
                char::from_u32(code).ok_or(LexError::InvalidEscape { pos })
            }
            _ => Err(LexError::InvalidEscape { pos }),
        }
    }

    fn scan_number(&mut self, first: char, start: u32) -> Result<TokenKind, LexError> {
        let mut text = String::from(first);
        let base = if first == '0' {
            match self.peek() {
                Some('x') | Some('X') => {
                    self.bump();
                    return self.scan_digits(16, start);
                }
                Some('b') | Some('B') => {
                    self.bump();
                    return self.scan_digits(2, start);
                }
                Some('o') | Some('O') => {
                    self.bump();
                    return self.scan_digits(8, start);
                }
                _ => 10,
            }
        } else {
            10
        };

        while let Some(c) = self.peek() {
            if c == '_' {
                self.bump();
                continue;
            }
            if c.is_ascii_digit() || (base == 16 && c.is_ascii_hexdigit()) {
                text.push(self.bump().unwrap());
            } else {
                break;
            }
        }

        if self.peek() == Some('.') && self.peek2().is_some_and(|c| c.is_ascii_digit()) {
            text.push(self.bump().unwrap());
            while let Some(c) = self.peek() {
                if c == '_' {
                    self.bump();
                    continue;
                }
                if c.is_ascii_digit() {
                    text.push(self.bump().unwrap());
                } else {
                    break;
                }
            }
            if matches!(self.peek(), Some('e' | 'E')) {
                text.push(self.bump().unwrap());
                if matches!(self.peek(), Some('+' | '-')) {
                    text.push(self.bump().unwrap());
                }
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    text.push(self.bump().unwrap());
                }
            }
            let v: f64 = text
                .replace('_', "")
                .parse()
                .map_err(|_| LexError::InvalidNumber { pos: start })?;
            return Ok(TokenKind::Float(v));
        }

        let clean = text.replace('_', "");
        let v: i64 = i64::from_str_radix(&clean, base)
            .map_err(|_| LexError::InvalidNumber { pos: start })?;
        Ok(TokenKind::Int(v))
    }

    fn scan_digits(&mut self, base: u32, start: u32) -> Result<TokenKind, LexError> {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c == '_' {
                self.bump();
                continue;
            }
            if c.is_ascii_digit() || (base == 16 && c.is_ascii_hexdigit()) {
                text.push(self.bump().unwrap());
            } else {
                break;
            }
        }
        if text.is_empty() {
            return Err(LexError::InvalidNumber { pos: start });
        }
        let v = i64::from_str_radix(&text.replace('_', ""), base)
            .map_err(|_| LexError::InvalidNumber { pos: start })?;
        Ok(TokenKind::Int(v))
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind;

    fn kinds(source: &str) -> Vec<TokenKind> {
        Lexer::new(source)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn lexes_hello_tokens() {
        let src = r#"greet(name) = "hello ""#;
        let toks = kinds(src);
        assert!(matches!(toks[0], TokenKind::Ident(_)));
        assert!(matches!(toks.last(), Some(TokenKind::Eof)));
    }

    #[test]
    fn lexes_keywords_and_operators() {
        let src = "if then else mut:= := |> &&& ||| ^^^ ** -> ?";
        let toks = kinds(src);
        assert!(toks.iter().any(|t| matches!(t, TokenKind::Kw(Kw::If))));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::MutColonEq)));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::ColonEq)));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::PipeGt)));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::AmpAmpAmp)));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::PipePipePipe)));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::CaretCaretCaret)));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::StarStar)));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::Arrow)));
        assert!(toks.iter().any(|t| matches!(t, TokenKind::Question)));
    }

    #[test]
    fn skips_comments() {
        let src = "x -- comment\n{- nested {- ok -} -} := 1";
        let toks = kinds(src);
        assert!(matches!(toks[0], TokenKind::Ident(_)));
        assert!(matches!(toks[1], TokenKind::ColonEq));
        assert!(matches!(toks[2], TokenKind::Int(1)));
    }

    #[test]
    fn lexes_numbers() {
        let src = "42 0xFF 0b1010 3.14 1_000";
        let toks = kinds(src);
        assert!(matches!(toks[0], TokenKind::Int(42)));
        assert!(matches!(toks[1], TokenKind::Int(255)));
        assert!(matches!(toks[2], TokenKind::Int(10)));
        assert!(matches!(toks[3], TokenKind::Float(_)));
        assert!(matches!(toks[4], TokenKind::Int(1000)));
    }
}
