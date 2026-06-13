//! Rust emission and rustc integration (spec §17.1, §17.3).

mod probe;
mod resolve;
mod rustc;

pub use probe::emit_probe_crate;
pub use resolve::{FallbackResolveError, resolve_rustc_fallbacks};
pub use rustc::{RustcError, check_rust_source, is_borrow_check_failure};

use anyhow::Result;
use crisp_cir::CirBuilder;

pub struct RustEmitter;

impl RustEmitter {
    pub fn emit_cargo_crate(_cir: &CirBuilder, _out_dir: &std::path::Path) -> Result<()> {
        todo!("emit target/rust/ Cargo project")
    }
}
