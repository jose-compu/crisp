//! Invoke cargo on emitted project (spec §17.1).

use crate::ice::map_rustc_failure;
use crate::project::emit_dir;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CargoError {
    #[error("failed to run cargo: {0}")]
    Io(#[from] std::io::Error),
    #[error("cargo not found on PATH")]
    NotFound,
    #[error("{0}")]
    BuildFailed(String),
    #[error("internal compiler error: generated Rust failed to compile at crisp span {span:?} — crpc bug (rustc: {summary})")]
    Ice {
        span: Option<crisp_ast::Span>,
        summary: String,
    },
}

pub fn cargo_check(crate_root: &Path, main_source: &str, source_map: &crate::source_map::EmitSourceMap) -> Result<(), CargoError> {
    run_cargo(crate_root, "check", main_source, source_map)
}

pub fn cargo_build(crate_root: &Path, main_source: &str, source_map: &crate::source_map::EmitSourceMap) -> Result<(), CargoError> {
    run_cargo(crate_root, "build", main_source, source_map)
}

fn run_cargo(
    crate_root: &Path,
    cmd: &str,
    main_source: &str,
    source_map: &crate::source_map::EmitSourceMap,
) -> Result<(), CargoError> {
    let out_dir = emit_dir(crate_root);
    let output = Command::new("cargo")
        .arg(cmd)
        .current_dir(&out_dir)
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let summary = stderr
                .lines()
                .find(|l| l.contains("error"))
                .unwrap_or("cargo build failed")
                .to_string();
            if let Some(span) = map_rustc_failure(&stderr, main_source, source_map) {
                return Err(CargoError::Ice {
                    span: Some(span),
                    summary,
                });
            }
            Err(CargoError::BuildFailed(summary))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(CargoError::NotFound),
        Err(e) => Err(CargoError::Io(e)),
    }
}

pub fn cargo_run(crate_root: &Path) -> Result<std::process::Output, CargoError> {
    let out_dir = emit_dir(crate_root);
    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .current_dir(&out_dir)
        .output();
    match output {
        Ok(out) if out.status.success() => Ok(out),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            Err(CargoError::BuildFailed(stderr))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(CargoError::NotFound),
        Err(e) => Err(CargoError::Io(e)),
    }
}

pub fn cargo_test(crate_root: &Path) -> Result<(), CargoError> {
    let out_dir = emit_dir(crate_root);
    let output = Command::new("cargo")
        .arg("test")
        .arg("--quiet")
        .current_dir(&out_dir)
        .output();
    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            Err(CargoError::BuildFailed(format!("{stdout}\n{stderr}")))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(CargoError::NotFound),
        Err(e) => Err(CargoError::Io(e)),
    }
}
