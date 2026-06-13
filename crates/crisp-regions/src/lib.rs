//! Region pass — lifetime inference and explicit emission (spec §8).

mod assign;
mod display;
mod lifetime;

pub use assign::{RegionError, RegionPass};
pub use display::{format_lifetime_sig, format_lifetimes_crate};
pub use lifetime::{LifetimeSig, RegionResult};
