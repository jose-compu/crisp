//! Resolve manifest dependencies to lock entries (spec §18).

use crate::lock::ResolvedDependency;
use crate::manifest::{CrateManifest, DependencySpec};

pub fn resolve_dependencies(manifest: &CrateManifest) -> Vec<ResolvedDependency> {
    let mut deps = Vec::new();

    if manifest.needs_tokio() {
        deps.push(ResolvedDependency {
            name: "tokio".into(),
            version: "1".into(),
            rust: true,
            features: vec!["rt".into(), "macros".into()],
        });
    }

    for (name, spec) in &manifest.dependencies {
        match spec {
            DependencySpec::Version(v) => {
                deps.push(ResolvedDependency {
                    name: name.clone(),
                    version: v.clone(),
                    rust: false,
                    features: Vec::new(),
                });
            }
            DependencySpec::Detailed {
                version,
                features,
                rust,
            } => {
                deps.push(ResolvedDependency {
                    name: name.clone(),
                    version: version.clone(),
                    rust: *rust,
                    features: features.clone(),
                });
            }
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    deps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::parse_manifest_str;

    #[test]
    fn resolves_tokio_and_rust_deps() {
        let m = parse_manifest_str(
            r#"
[package]
name = "x"
version = "0.1.0"
edition = "2026"
[build]
runtime = "tokio"
[dependencies]
serde_json = { rust = true, version = "1" }
"#,
        )
        .unwrap();
        let deps = resolve_dependencies(&m);
        assert!(deps.iter().any(|d| d.name == "tokio" && d.rust));
        assert!(deps.iter().any(|d| d.name == "serde_json" && d.rust));
    }
}
