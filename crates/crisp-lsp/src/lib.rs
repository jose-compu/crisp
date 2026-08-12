//! LSP **analysis** layer — ghost-text hints, hover, ownership overlays (spec §16.3).
//!
//! # Status (v1.5)
//!
//! - Library: [`CrispAnalysis`] for tests and custom hosts.
//! - Stdio server: binary `crisp-lsp` ([#56](https://github.com/jose-compu/crisp/issues/56)).
//!
//! ```bash
//! cargo run -p crisp-lsp --bin crisp-lsp
//! # or: cargo install --path crates/crisp-lsp --locked
//! ```
//!
//! # Example (library)
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
pub use hints::{InlayHint, InlayHintKind};
pub use hover::HoverInfo;
pub use lenses::CodeLens;
pub use overlays::CallOverlay;
