//! Diagnostic reporting against Crisp source spans.

mod format;

use crisp_ast::Span;
use thiserror::Error;

pub use format::{
    FormattedDiagnostic, format_diagnostic, format_ownership_contradiction, format_type_mismatch,
    from_diagnostic,
};

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
    #[error("internal compiler error: generated Rust failed to compile — crpc bug")]
    InternalIce,
}

pub struct DiagnosticSink {
    pub diagnostics: Vec<Diagnostic>,
}
