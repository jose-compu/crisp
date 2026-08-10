//! LSP **analysis** layer — ghost-text hints, hover, ownership overlays (spec §16.3).
//!
//! # Status (v1.3 track)
//!
//! This crate ships the editor-facing **analysis API** used by tests and future hosts.
//! There is **no** stdio/`tower-lsp` server binary yet ([#18](https://github.com/jose-compu/crisp/issues/18)).
//! Until one lands, call [`CrispAnalysis`] from an IDE extension or use the `reveal` / `crpc`
//! CLIs (see QUICKSTART §10–§11).
//!
//! # Example
//!
//! ```no_run
//! use crisp_lsp::CrispAnalysis;
//! use std::path::Path;
//!
//! let root = Path::new("examples/hello");
//! let analysis = CrispAnalysis::analyze(root).expect("analyze");
//! let file = root.join("src/main.crp");
//! let _hints = analysis.inlay_hints(&file).expect("hints");
//! let _hover = analysis.hover(&file, 0).expect("hover");
//! ```

mod analysis;
mod hints;
mod hover;
mod lenses;
mod overlays;
mod walk;

pub use analysis::{AnalysisError, CrispAnalysis};
pub use hints::InlayHint;
pub use hover::HoverInfo;
pub use lenses::CodeLens;
pub use overlays::CallOverlay;

/// Placeholder for a future stdio/TCP LSP server host ([#18](https://github.com/jose-compu/crisp/issues/18)).
///
/// Prefer [`CrispAnalysis`] until a real protocol host is wired.
pub struct CrispLanguageServer;
