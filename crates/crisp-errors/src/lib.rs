//! Error pass — ambient propagation, reachable CrispError sets (spec §9).

mod analyze;
mod display;
mod error;
mod result;
mod set;

pub use analyze::ErrorPass;
pub use display::{format_crisp_error_enum, format_error_sig, format_errors_crate};
pub use error::ErrorPassError;
pub use result::{CrispErrorEnum, CrispErrorVariant, ErrorResult, ErrorSet, ErrorSig};
