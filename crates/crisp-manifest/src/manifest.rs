//! Parse `crisp.toml` (spec §18.2).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to read manifest: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse crisp.toml: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("missing [package] section in crisp.toml")]
    MissingPackage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateManifest {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub build: BuildSection,
    pub dependencies: BTreeMap<String, DependencySpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuildSection {
    pub target: String,
    pub runtime: Option<String>,
    pub error_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySpec {
    Version(String),
    Detailed {
        version: String,
        features: Vec<String>,
        rust: bool,
    },
}

impl CrateManifest {
    pub fn rust_edition(&self) -> String {
        map_rust_edition(&self.edition)
    }

    pub fn needs_tokio(&self) -> bool {
        self.build
            .runtime
            .as_deref()
            .is_some_and(|r| r == "tokio")
    }
}

pub fn read_manifest(crate_root: &Path) -> Result<CrateManifest, ManifestError> {
    let raw = fs::read_to_string(crate_root.join("crisp.toml"))?;
    parse_manifest_str(&raw)
}

pub fn parse_manifest_str(raw: &str) -> Result<CrateManifest, ManifestError> {
    let table: toml::Table = toml::from_str(raw).map_err(ManifestError::Parse)?;

    let package = table
        .get("package")
        .and_then(|v| v.as_table())
        .ok_or(ManifestError::MissingPackage)?;

    let name = package
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("crisp_app")
        .to_string();
    let version = package
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();
    let edition = package
        .get("edition")
        .and_then(|v| v.as_str())
        .unwrap_or("2026")
        .to_string();

    let build = table
        .get("build")
        .and_then(|v| v.as_table())
        .map(parse_build_section)
        .unwrap_or_default();

    let dependencies = table
        .get("dependencies")
        .and_then(|v| v.as_table())
        .map(parse_dependencies)
        .unwrap_or_default();

    Ok(CrateManifest {
        name,
        version,
        edition,
        build,
        dependencies,
    })
}

fn parse_build_section(table: &toml::Table) -> BuildSection {
    BuildSection {
        target: table
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("rust")
            .to_string(),
        runtime: table.get("runtime").and_then(|v| v.as_str()).map(str::to_string),
        error_model: table
            .get("error_model")
            .and_then(|v| v.as_str())
            .unwrap_or("enum")
            .to_string(),
    }
}

fn parse_dependencies(table: &toml::Table) -> BTreeMap<String, DependencySpec> {
    let mut deps = BTreeMap::new();
    for (name, value) in table {
        let spec = match value {
            toml::Value::String(v) => DependencySpec::Version(v.clone()),
            toml::Value::Table(t) => {
                let version = t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .to_string();
                let features = t
                    .get("features")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|f| f.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let rust = t
                    .get("rust")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                DependencySpec::Detailed {
                    version,
                    features,
                    rust,
                }
            }
            _ => continue,
        };
        deps.insert(name.clone(), spec);
    }
    deps
}

fn map_rust_edition(crisp_edition: &str) -> String {
    match crisp_edition {
        "2026" | "2024" => "2021".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[package]
name = "my_project"
version = "0.1.0"
edition = "2026"

[build]
target = "rust"
runtime = "tokio"
error_model = "enum"

[dependencies]
http = "1.2"
json = { version = "0.5", features = ["pretty"] }
serde_json = { rust = true, version = "1" }
"#;

    #[test]
    fn parse_full_manifest() {
        let m = parse_manifest_str(SAMPLE).expect("parse");
        assert_eq!(m.name, "my_project");
        assert_eq!(m.edition, "2026");
        assert_eq!(m.rust_edition(), "2021");
        assert!(m.needs_tokio());
        assert_eq!(m.build.error_model, "enum");
        assert_eq!(m.dependencies.len(), 3);
        assert!(matches!(
            m.dependencies.get("http"),
            Some(DependencySpec::Version(v)) if v == "1.2"
        ));
        assert!(matches!(
            m.dependencies.get("serde_json"),
            Some(DependencySpec::Detailed { rust: true, .. })
        ));
    }

    #[test]
    fn read_hello_manifest() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
        let m = read_manifest(&root).expect("read");
        assert_eq!(m.name, "hello");
        assert!(m.needs_tokio());
    }

    use std::path::PathBuf;
}
