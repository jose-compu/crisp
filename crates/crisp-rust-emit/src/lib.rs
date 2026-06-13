//! Rust emission and rustc integration (spec §17.1, §17.3).

mod cargo;
mod emit;
mod fallible;
mod ice;
mod pipeline;
mod probe;
mod project;
mod resolve;
mod rustc;
mod source_map;

pub use emit::{emit_crate, format_ty, EmitResult};
pub use fallible::emit_fallible_probe_crate;
pub use pipeline::{
    EmitOutput, PipelineError, analyze_and_build_cir, build_emitted, check_emitted, emit_to_target,
    run_emitted,
};
pub use probe::emit_probe_crate;
pub use project::{emit_dir, read_manifest, write_cargo_project};
pub use resolve::{FallbackResolveError, resolve_rustc_fallbacks};
pub use rustc::{RustcError, check_rust_source, is_borrow_check_failure};
pub use source_map::EmitSourceMap;

use anyhow::Result;
use crisp_cir::CirCrate;
use std::path::Path;

pub struct RustEmitter;

impl RustEmitter {
    pub fn emit_cargo_crate(cir: &CirCrate, crate_root: &Path) -> Result<EmitOutput> {
        let manifest = read_manifest(crate_root)?;
        let emitted = emit_crate(cir);
        let out_dir = write_cargo_project(crate_root, &emitted, &manifest)?;
        Ok(EmitOutput {
            cir: cir.clone(),
            main_rs: emitted.lib_rs,
            out_dir,
            source_map: emitted.source_map,
        })
    }
}
