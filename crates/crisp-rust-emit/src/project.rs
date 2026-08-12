//! Write emitted Rust to `target/rust/` Cargo project (spec §18).

use crate::emit::EmitResult;
use anyhow::{Context, Result};
use crisp_manifest::{CrateManifest, ResolvedDependency};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

pub fn emit_dir(crate_root: &Path) -> PathBuf {
    crate_root.join("target").join("rust")
}

type EmitLocks = Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>;

fn emit_locks() -> &'static EmitLocks {
    static LOCKS: OnceLock<EmitLocks> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Serialize emit/build/test IO for a Crisp crate's `target/rust` directory.
///
/// Integration tests often call `build_emitted` / `run_emitted` / `run_tests` on the
/// same example in parallel; without this lock, concurrent writers truncate `Cargo.toml`.
pub fn with_emit_dir_lock<T>(crate_root: &Path, f: impl FnOnce() -> T) -> T {
    let key = emit_dir(crate_root);
    let lock = {
        let mut map = emit_locks().lock().unwrap_or_else(|e| e.into_inner());
        map.entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

pub fn write_cargo_project(
    crate_root: &Path,
    emitted: &EmitResult,
    manifest: &CrateManifest,
    extra_deps: &[ResolvedDependency],
    test_rs: Option<&str>,
) -> Result<PathBuf> {
    with_emit_dir_lock(crate_root, || {
        write_cargo_project_unlocked(crate_root, emitted, manifest, extra_deps, test_rs)
    })
}

pub(crate) fn write_cargo_project_unlocked(
    crate_root: &Path,
    emitted: &EmitResult,
    manifest: &CrateManifest,
    extra_deps: &[ResolvedDependency],
    test_rs: Option<&str>,
) -> Result<PathBuf> {
    let out_dir = emit_dir(crate_root);
    let src = out_dir.join("src");
    fs::create_dir_all(&src)?;

    let cargo_toml = format_cargo_toml(manifest, extra_deps);
    atomic_write(&out_dir.join("Cargo.toml"), &cargo_toml)?;

    let mut main_rs = emitted.lib_rs.clone();
    if let Some(tests) = test_rs {
        main_rs.push_str(tests);
    }
    atomic_write(&src.join("main.rs"), &main_rs)?;

    for (mod_name, content) in &emitted.modules {
        // Dotted Crisp paths (`math.vector`) → nested Rust files (`math/vector.rs`).
        let rel = mod_name.replace('.', "/");
        let path = src.join(format!("{rel}.rs"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&path, content)?;
    }

    Ok(out_dir)
}

fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("temp file for {}", path.display()))?;
    tmp.write_all(contents.as_bytes())?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("persist {}", path.display()))?;
    Ok(())
}

fn format_cargo_toml(manifest: &CrateManifest, extra_deps: &[ResolvedDependency]) -> String {
    let mut out = format!(
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
        edition = manifest.rust_edition(),
    );

    let mut deps: Vec<&ResolvedDependency> = extra_deps.iter().collect();
    deps.sort_by(|a, b| a.name.cmp(&b.name));
    if !deps.is_empty() {
        out.push_str("\n[dependencies]\n");
        for dep in deps {
            out.push_str(&format_dep_line(dep));
        }
    }
    out
}

fn format_dep_line(dep: &ResolvedDependency) -> String {
    if dep.features.is_empty() {
        format!("{} = \"{}\"\n", dep.name, dep.version)
    } else {
        let features = dep
            .features
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} = {{ version = \"{}\", features = [{}] }}\n",
            dep.name, dep.version, features
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crisp_manifest::{read_manifest, resolve_dependencies};

    #[test]
    fn cargo_toml_includes_tokio_from_manifest() {
        let m = read_manifest(&hello_root()).unwrap();
        let deps = resolve_dependencies(&m);
        let toml = format_cargo_toml(&m, &deps);
        assert!(toml.contains("name = \"hello\""));
        assert!(toml.contains("tokio"));
        assert!(toml.contains("features"));
    }

    fn hello_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
    }
}
