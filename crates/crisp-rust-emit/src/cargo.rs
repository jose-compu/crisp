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
    #[error(
        "internal compiler error: generated Rust failed to compile at crisp span {span:?} — crisp bug (rustc: {summary})"
    )]
    Ice {
        span: Option<crisp_ast::Span>,
        summary: String,
    },
}

pub fn cargo_check(
    crate_root: &Path,
    main_source: &str,
    source_map: &crate::source_map::EmitSourceMap,
) -> Result<(), CargoError> {
    run_cargo(crate_root, "check", main_source, source_map)
}

pub fn cargo_build(
    crate_root: &Path,
    main_source: &str,
    source_map: &crate::source_map::EmitSourceMap,
) -> Result<(), CargoError> {
    run_cargo(crate_root, "build", main_source, source_map)
}

/// Cargo invoked against `target/rust/Cargo.toml` with cwd = Crisp crate root (#106).
fn cargo_command(crate_root: &Path, subcommand: &str) -> Command {
    let manifest = emit_dir(crate_root).join("Cargo.toml");
    let mut cmd = Command::new("cargo");
    cmd.arg(subcommand);
    cmd.arg("--manifest-path").arg(&manifest);
    cmd.current_dir(crate_root);
    match crate_root.canonicalize() {
        Ok(root) => {
            cmd.env("CRISP_CRATE_ROOT", root);
        }
        Err(_) => {
            cmd.env("CRISP_CRATE_ROOT", crate_root);
        }
    }
    cmd
}

fn run_cargo(
    crate_root: &Path,
    cmd: &str,
    main_source: &str,
    source_map: &crate::source_map::EmitSourceMap,
) -> Result<(), CargoError> {
    let output = cargo_command(crate_root, cmd).output();

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
    let output = cargo_command(crate_root, "run").arg("--quiet").output();
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
    let output = cargo_command(crate_root, "test").arg("--quiet").output();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn write_probe_project(root: &Path) {
        let emit = emit_dir(root);
        fs::create_dir_all(emit.join("src")).unwrap();
        fs::write(
            emit.join("Cargo.toml"),
            r#"[package]
name = "cwd_probe"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "cwd_probe"
path = "src/main.rs"

[workspace]
"#,
        )
        .unwrap();
        fs::write(
            emit.join("src/main.rs"),
            r#"fn main() {
    let cwd = std::env::current_dir().unwrap();
    println!("cwd={}", cwd.display());
    println!(
        "crate_root={}",
        std::env::var("CRISP_CRATE_ROOT").unwrap_or_default()
    );
    println!("has_git={}", std::path::Path::new(".git").exists());
}
"#,
        )
        .unwrap();
    }

    #[test]
    fn cargo_run_cwd_is_crate_root_not_emit_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        write_probe_project(root);

        let git = Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output();
        match git {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("SKIP #106: git not on PATH");
                return;
            }
            Err(e) => panic!("git init: {e}"),
            Ok(out) if !out.status.success() => {
                eprintln!(
                    "SKIP #106: git init failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                return;
            }
            Ok(_) => {}
        }

        match cargo_run(root) {
            Err(CargoError::NotFound) => {
                eprintln!("SKIP #106: cargo not on PATH");
            }
            Err(e) => panic!("cargo_run: {e}"),
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                eprintln!("#106 stdout:\n{stdout}");
                let canon = root.canonicalize().unwrap();
                let canon_s = canon.to_string_lossy();
                assert!(
                    stdout.contains(&format!("cwd={canon_s}")),
                    "cwd should be crate root {canon_s}, got:\n{stdout}"
                );
                assert!(
                    !stdout.contains("target/rust"),
                    "cwd must not be the emit dir:\n{stdout}"
                );
                assert!(
                    stdout.contains(&format!("crate_root={canon_s}")),
                    "CRISP_CRATE_ROOT:\n{stdout}"
                );
                assert!(
                    stdout.contains("has_git=true"),
                    ".git at crate root should be visible:\n{stdout}"
                );
            }
        }
    }

    #[test]
    fn cargo_command_points_at_emitted_manifest() {
        let root = PathBuf::from("/tmp/crisp-app");
        let cmd = cargo_command(&root, "run");
        let debug = format!("{cmd:?}");
        assert!(
            debug.contains("target") && debug.contains("rust") && debug.contains("Cargo.toml"),
            "manifest-path missing: {debug}"
        );
    }
}
