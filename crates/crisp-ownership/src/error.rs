use crisp_ast::Span;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OwnershipError {
    #[error("[E0050] ownership contradicts annotation on `{name}`: inferred `{inferred}`, annotated `{annotated}`")]
    ContradictsAnnotation {
        name: String,
        inferred: String,
        annotated: String,
        span: Span,
    },
    #[error("[E0051] ownership analysis failed: {message}")]
    Internal { message: String },
    #[error("[E0052] resolve error: {0}")]
    Resolve(#[from] crisp_resolve::ResolveError),
    #[error("[E0053] type error: {0}")]
    Type(#[from] crisp_typeck::TypeError),
}
