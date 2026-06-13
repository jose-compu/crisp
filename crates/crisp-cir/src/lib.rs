//! CIR — typed IR with resolved ownership modes (spec §17.1).

pub mod build;
pub mod node;
pub mod source_map;
pub mod synthesize;
pub mod ty;

pub use build::{CirBuilder, CirError};
pub use node::*;
pub use source_map::SourceMap;
pub use ty::CirTy;
