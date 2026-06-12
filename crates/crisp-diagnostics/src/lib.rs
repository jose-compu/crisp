//! Diagnostic reporting against Crisp source spans.

use crisp_ast::Span;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub span: Span,
    pub severity: Severity,
}

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("[{code}] {message}")]
    User { code: String, message: String },
    #[error("internal compiler error: generated Rust failed to compile — crispc bug")]
    InternalIce,
}

pub struct DiagnosticSink {
    pub diagnostics: Vec<Diagnostic>,
}
