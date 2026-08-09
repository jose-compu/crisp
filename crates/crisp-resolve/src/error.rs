use crisp_ast::Span;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("module not found: {path}")]
    ModuleNotFound { path: String },
    #[error("crate has no src/ directory at {root}")]
    NoSrcDir { root: String },
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error in {path}: {message}")]
    Parse { path: String, message: String },
    #[error("[E0034] duplicate definition of `{name}` in module `{module}`")]
    DuplicateDef {
        name: String,
        module: String,
        span: Span,
    },
    #[error("{message}")]
    UnresolvedName {
        name: String,
        span: Span,
        /// Full user-facing message including `[E0035]` and optional help.
        message: String,
        hint: Option<String>,
    },
    #[error("[E0036] `{name}` is private in module `{module}`")]
    PrivateImport {
        name: String,
        module: String,
        span: Span,
    },
    #[error("[E0037] symbol `{name}` not exported from module `{module}`")]
    NotExported {
        name: String,
        module: String,
        span: Span,
    },
    #[error("[E0038] ambiguous import: `{name}` defined in multiple modules")]
    AmbiguousImport { name: String, span: Span },
    #[error(
        "[E0039] shapes are not yet supported (`{name}`); remove the `shape` definition or bound (tracked: #21)"
    )]
    ShapesUnsupported { name: String, span: Span },
}
