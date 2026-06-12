//! Rust emission and rustc integration (spec §17.1, §17.3).

use anyhow::Result;
use crisp_cir::CirBuilder;

pub struct RustEmitter;

impl RustEmitter {
    pub fn emit_cargo_crate(_cir: &CirBuilder, _out_dir: &std::path::Path) -> Result<()> {
        todo!("emit target/rust/ Cargo project")
    }
}
