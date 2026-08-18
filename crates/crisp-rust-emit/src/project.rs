//! Write emitted Rust to `target/rust/` Cargo project (spec §18).

use crate::emit::EmitResult;
use anyhow::{Context, Result};
use crisp_manifest::{CrateManifest, ResolvedDependency};
use std::collections::{BTreeMap, HashMap};
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
    let map = test_rs.map(|s| {
        let mut m = BTreeMap::new();
        m.insert("main".to_string(), s.to_string());
        m
    });
    with_emit_dir_lock(crate_root, || {
        write_cargo_project_unlocked(crate_root, emitted, manifest, extra_deps, map.as_ref())
    })
}

pub(crate) fn write_cargo_project_unlocked(
    crate_root: &Path,
    emitted: &EmitResult,
    manifest: &CrateManifest,
    extra_deps: &[ResolvedDependency],
    tests_by_module: Option<&BTreeMap<String, String>>,
) -> Result<PathBuf> {
    let out_dir = emit_dir(crate_root);
    let src = out_dir.join("src");
    fs::create_dir_all(&src)?;

    let cargo_toml = format_cargo_toml(manifest, extra_deps, crate_root);
    atomic_write(&out_dir.join("Cargo.toml"), &cargo_toml)?;

    let mut main_rs = emitted.lib_rs.clone();
    if let Some(map) = tests_by_module
        && let Some(tests) = map.get("main")
    {
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
        let mut body = content.clone();
        if let Some(map) = tests_by_module
            && let Some(tests) = map.get(mod_name)
        {
            body.push_str(tests);
        }
        atomic_write(&path, &body)?;
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

fn format_cargo_toml(
    manifest: &CrateManifest,
    extra_deps: &[ResolvedDependency],
    crate_root: &Path,
) -> String {
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
            out.push_str(&format_dep_line(dep, crate_root));
        }
    }
    out
}

fn format_dep_line(dep: &ResolvedDependency, crate_root: &Path) -> String {
    if let Some(spec_path) = &dep.path {
        let rel = path_dep_for_emit(crate_root, spec_path);
        let mut fields = vec![format!("path = \"{}\"", toml_escape(&rel))];
        if !dep.version.is_empty() && dep.version != "*" {
            fields.push(format!("version = \"{}\"", toml_escape(&dep.version)));
        }
        if !dep.features.is_empty() {
            let features = dep
                .features
                .iter()
                .map(|f| format!("\"{f}\""))
                .collect::<Vec<_>>()
                .join(", ");
            fields.push(format!("features = [{features}]"));
        }
        return format!("{} = {{ {} }}\n", dep.name, fields.join(", "));
    }
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

fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Rewrite a `crisp.toml` path (relative to the Crisp crate root) so Cargo can
/// resolve it from `target/rust/Cargo.toml` (#105).
pub(crate) fn path_dep_for_emit(crate_root: &Path, spec_path: &str) -> String {
    let spec = Path::new(spec_path);
    if spec.is_absolute() {
        return spec.to_string_lossy().replace('\\', "/");
    }
    let emit = crate_root.join("target").join("rust");
    let dest = crate_root.join(spec);
    relative_path(&emit, &dest)
        .to_string_lossy()
        .replace('\\', "/")
}

fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn relative_path(from_dir: &Path, to: &Path) -> PathBuf {
    let from = lexical_normalize(from_dir);
    let to = lexical_normalize(to);
    let from_c: Vec<_> = from.components().collect();
    let to_c: Vec<_> = to.components().collect();
    let mut i = 0;
    while i < from_c.len() && i < to_c.len() && from_c[i] == to_c[i] {
        i += 1;
    }
    let mut rel = PathBuf::new();
    for _ in i..from_c.len() {
        rel.push("..");
    }
    for c in &to_c[i..] {
        rel.push(c.as_os_str());
    }
    if rel.as_os_str().is_empty() {
        rel.push(".");
    }
    rel
}

#[cfg(test)]
mod tests {
    use super::*;
    use crisp_manifest::{read_manifest, resolve_dependencies};

    #[test]
    fn cargo_toml_includes_tokio_from_manifest() {
        let m = read_manifest(&hello_root()).unwrap();
        let deps = resolve_dependencies(&m);
        let toml = format_cargo_toml(&m, &deps, &hello_root());
        assert!(toml.contains("name = \"hello\""));
        assert!(toml.contains("tokio"));
        assert!(toml.contains("features"));
    }

    #[test]
    fn path_dep_is_rewritten_relative_to_emit_dir() {
        let crate_root = PathBuf::from("/tmp/app");
        assert_eq!(
            path_dep_for_emit(&crate_root, "../local_core"),
            "../../../local_core"
        );
        assert_eq!(
            path_dep_for_emit(&crate_root, "vendor/local_core"),
            "../../vendor/local_core"
        );
        assert_eq!(path_dep_for_emit(&crate_root, "/abs/core"), "/abs/core");
    }

    fn hello_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
    }
}
