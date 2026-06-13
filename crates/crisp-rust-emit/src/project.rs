//! Write emitted Rust to `target/rust/` Cargo project (spec §18).

use crate::emit::EmitResult;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CrateManifest {
    pub name: String,
    pub version: String,
    pub edition: String,
}

pub fn read_manifest(crate_root: &Path) -> Result<CrateManifest> {
    let raw = fs::read_to_string(crate_root.join("crisp.toml"))
        .with_context(|| format!("read {}", crate_root.join("crisp.toml").display()))?;
    let table: toml::Table = raw.parse().context("parse crisp.toml")?;
    let package = table.get("package").context("[package] section")?;
    Ok(CrateManifest {
        name: package
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("crisp_app")
            .to_string(),
        version: package
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.1.0")
            .to_string(),
        edition: package
            .get("edition")
            .and_then(|v| v.as_str())
            .map(map_rust_edition)
            .unwrap_or_else(|| "2021".to_string()),
    })
}

pub fn emit_dir(crate_root: &Path) -> PathBuf {
    crate_root.join("target").join("rust")
}

pub fn write_cargo_project(
    crate_root: &Path,
    emitted: &EmitResult,
    manifest: &CrateManifest,
) -> Result<PathBuf> {
    let out_dir = emit_dir(crate_root);
    let src = out_dir.join("src");
    fs::create_dir_all(&src)?;

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "{version}"
edition = "{edition}"

[[bin]]
name = "{name}"
path = "src/main.rs"

[workspace]
"#,
        name = manifest.name,
        version = manifest.version,
        edition = manifest.edition,
    );
    fs::write(out_dir.join("Cargo.toml"), cargo_toml)?;

    fs::write(src.join("main.rs"), &emitted.lib_rs)?;
    for (mod_name, content) in &emitted.modules {
        fs::write(src.join(format!("{mod_name}.rs")), content)?;
    }

    Ok(out_dir)
}

fn map_rust_edition(crisp_edition: &str) -> String {
    match crisp_edition {
        "2026" | "2024" => "2021".to_string(),
        other => other.to_string(),
    }
}
