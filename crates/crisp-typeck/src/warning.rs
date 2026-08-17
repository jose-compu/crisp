//! Typeck warnings (non-fatal).

use crisp_ast::Span;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum TypeWarning {
    #[error(
        "[W0087] converting `int` to `float` may lose precision above 2^53; write `as float` to silence"
    )]
    IntToFloat { span: Span },
}

impl TypeWarning {
    pub fn code(&self) -> &'static str {
        match self {
            Self::IntToFloat { .. } => "W0087",
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Self::IntToFloat { span } => *span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w0087_code() {
        let w = TypeWarning::IntToFloat {
            span: Span::new(0, 1),
        };
        assert_eq!(w.code(), "W0087");
        assert!(w.to_string().contains("W0087"));
    }
}
