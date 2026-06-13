//! LSP analysis layer — ghost-text hints, hover, ownership overlays (spec §16.3).

mod analysis;
mod hover;
mod hints;
mod lenses;
mod overlays;
mod walk;

pub use analysis::{AnalysisError, CrispAnalysis};
pub use hover::HoverInfo;
pub use hints::InlayHint;
pub use lenses::CodeLens;
pub use overlays::CallOverlay;

/// Placeholder for a future stdio/TCP LSP server host.
pub struct CrispLanguageServer;
