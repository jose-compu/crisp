//! LSP analysis layer — ghost-text hints, hover, ownership overlays (spec §16.3).

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

/// Placeholder for a future stdio/TCP LSP server host.
pub struct CrispLanguageServer;
