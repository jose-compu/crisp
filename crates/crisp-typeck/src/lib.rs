//! HM-style type inference and constraint solving (spec §3.4).

mod display;
mod env;
mod infer;
mod types;
mod unify;
mod warning;

pub use display::{format_sig, format_ty};
pub use infer::{NumericCoercion, TypeChecker, TypeError, TypedCrate, rust_import_returns_result};
pub use types::{InferredSig, Ty, is_arith_bound, rust_op_bound};
pub use warning::TypeWarning;
