use crisp_ast::Span;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorPassError {
    #[error("[E0070] function `{name}` asserts `!never` but may produce `{produced}`")]
    NeverViolated {
        name: String,
        produced: String,
        span: Span,
    },
    #[error("[E0071] function `{name}` declares `!{declared}` but body may produce `{produced}`")]
    DeclaredMismatch {
        name: String,
        declared: String,
        produced: String,
        span: Span,
    },
    #[error("[E0072] resolve error: {0}")]
    Resolve(#[from] crisp_resolve::ResolveError),
    #[error("[E0073] type error: {0}")]
    Type(#[from] crisp_typeck::TypeError),
}
