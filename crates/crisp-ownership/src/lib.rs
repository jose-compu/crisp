//! Ownership pass — usage collection, lattice join, clone insertion (spec §7).

mod analyze;
mod display;
mod error;
mod fallback;
mod lattice;
mod result;
mod usage;

pub use analyze::OwnershipPass;
pub use display::{format_owned_sig, format_ownership_crate};
pub use error::OwnershipError;
pub use fallback::{apply_fallback, candidates_for_auto_clone, fallback_chain};
pub use lattice::OwnershipMode;
pub use result::{AppliedFallback, AutoClone, FallbackKind, OwnershipResult, OwnershipSignature};
