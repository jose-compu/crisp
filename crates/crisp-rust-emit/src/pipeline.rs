//! Full analyze → CIR → emit → cargo pipeline.

use crate::cargo::{CargoError, cargo_build, cargo_check, cargo_run};
use crate::emit::emit_crate;
use crate::project::{emit_dir, write_cargo_project};
use crate::resolve::resolve_rustc_fallbacks;
use crate::seal::verify_sealed_api;
use crate::source_map::EmitSourceMap;
use anyhow::{Context, Result};
use crisp_cir::{CirBuilder, CirCrate, CirError};
use crisp_manifest::{read_manifest, resolve_dependencies};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("{0}")]
    Cir(#[from] CirError),
    #[error("{0}")]
    Cargo(#[from] CargoError),
    #[error("{0}")]
    Seal(#[from] crate::seal::SealDriftError),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
    #[error("[E0058] rustc/cargo not available")]
    ToolchainUnavailable,
}

pub struct EmitOutput {
    pub cir: CirCrate,
    pub main_rs: String,
    pub out_dir: std::path::PathBuf,
    pub source_map: EmitSourceMap,
}

pub fn analyze_and_build_cir(crate_root: &Path) -> Result<CirCrate, PipelineError> {
    verify_sealed_api(crate_root)?;
    let _ = resolve_rustc_fallbacks(crate_root);
    Ok(CirBuilder::build_crate(crate_root)?)
}

pub fn emit_to_target(crate_root: &Path) -> Result<EmitOutput, PipelineError> {
    let cir = analyze_and_build_cir(crate_root)?;
    let manifest = read_manifest(crate_root).context("read crisp.toml")?;
    let deps = resolve_dependencies(&manifest);
    let emitted = emit_crate(&cir);
    let out_dir = write_cargo_project(crate_root, &emitted, &manifest, &deps, None)
        .context("write target/rust")?;
    Ok(EmitOutput {
        cir,
        main_rs: emitted.lib_rs,
        out_dir,
        source_map: emitted.source_map,
    })
}

pub fn check_emitted(crate_root: &Path) -> Result<(), PipelineError> {
    let out = emit_to_target(crate_root)?;
    match cargo_check(crate_root, &out.main_rs, &out.source_map) {
        Err(CargoError::NotFound) => Err(PipelineError::ToolchainUnavailable),
        other => other.map_err(PipelineError::from),
    }
}

pub fn build_emitted(crate_root: &Path) -> Result<std::path::PathBuf, PipelineError> {
    let out = emit_to_target(crate_root)?;
    match cargo_build(crate_root, &out.main_rs, &out.source_map) {
        Err(CargoError::NotFound) => Err(PipelineError::ToolchainUnavailable),
        Ok(()) => Ok(emit_dir(crate_root)),
        Err(e) => Err(PipelineError::from(e)),
    }
}

pub fn run_emitted(crate_root: &Path) -> Result<String, PipelineError> {
    build_emitted(crate_root)?;
    match cargo_run(crate_root) {
        Err(CargoError::NotFound) => Err(PipelineError::ToolchainUnavailable),
        Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).into_owned()),
        Err(e) => Err(PipelineError::from(e)),
    }
}
