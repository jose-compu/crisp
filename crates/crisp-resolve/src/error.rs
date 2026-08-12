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
    #[error(
        "[E0044] Rust crate `{name}` is not a dependency; add it under `[dependencies]` in crisp.toml with `rust = true` (spec §14.2, #41)"
    )]
    RustCrateNotFound { name: String, span: Span },
    #[error(
        "[E0045] dependency `{name}` must set `rust = true` to import via `use rust…` (spec §14.2, #41)"
    )]
    RustCrateNotMarked { name: String, span: Span },
    #[error(
        "[E0046] `use rust…` requires an import list, e.g. `use rust.{name} {{ item }}` (spec §14.2)"
    )]
    RustImportNeedsList { name: String, span: Span },
    #[error("[E0047] invalid `use rust` path `{path}`; expected `use rust.<crate> {{ … }}`")]
    RustUsePathInvalid { path: String, span: Span },
    #[error("failed to read crisp.toml at {root}: {message}")]
    Manifest { root: String, message: String },
}
