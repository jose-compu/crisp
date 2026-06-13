//! Ownership pass — usage collection, lattice join, clone insertion (spec §7).

mod analyze;
mod display;
mod error;
mod lattice;
mod result;
mod usage;

pub use analyze::OwnershipPass;
pub use display::{format_owned_sig, format_ownership_crate};
pub use error::OwnershipError;
pub use lattice::OwnershipMode;
pub use result::{AutoClone, OwnershipResult, OwnershipSignature};
