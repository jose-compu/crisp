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
mod seal;
mod source_map;
mod test_harness;

pub use crisp_manifest::read_manifest;
pub use emit::{EmitResult, emit_crate, format_ty};
pub use fallible::emit_fallible_probe_crate;
pub use pipeline::{
    EmitOutput, PipelineError, analyze_and_build_cir, build_emitted, check_emitted, emit_to_target,
    run_emitted,
};
pub use probe::emit_probe_crate;
pub use project::{emit_dir, with_emit_dir_lock, write_cargo_project};
pub use resolve::{FallbackResolveError, resolve_rustc_fallbacks};
pub use rustc::{RustcError, check_rust_source, is_borrow_check_failure};
pub use seal::{
    SealDriftError, compute_sealed_api, format_sealed_api, update_lock, verify_sealed_api,
};
pub use source_map::EmitSourceMap;
pub use test_harness::{
    CollectedTest, TestHarnessError, TestRunReport, collect_tests, emit_test_module, run_tests,
};

use anyhow::Result;
use crisp_cir::CirCrate;
use std::path::Path;

pub struct RustEmitter;

impl RustEmitter {
    pub fn emit_cargo_crate(cir: &CirCrate, crate_root: &Path) -> Result<EmitOutput> {
        let manifest = read_manifest(crate_root)?;
        let deps = crisp_manifest::resolve_dependencies(&manifest);
        let emitted = emit_crate(cir);
        let out_dir = write_cargo_project(crate_root, &emitted, &manifest, &deps, None)?;
        Ok(EmitOutput {
            cir: cir.clone(),
            main_rs: emitted.lib_rs,
            out_dir,
            source_map: emitted.source_map,
        })
    }
}
