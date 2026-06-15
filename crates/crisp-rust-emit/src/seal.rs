//! Compute and verify sealed `pub` API signatures (spec §12.5).

use crisp_ast::item::{Item, TypeBody};
use crisp_errors::{ErrorPass, ErrorSig};
use crisp_manifest::{
    CrispLock, LOCK_VERSION, SealedSignature, read_lock, read_manifest, resolve_dependencies,
    write_lock,
};
use crisp_ownership::format_owned_sig;
use crisp_ownership::{OwnershipPass, OwnershipSignature};
use crisp_regions::RegionPass;
use crisp_resolve::module::load_module_graph;
use crisp_resolve::symbols::{Visibility, collect_module_symbols};
use crisp_typeck::{InferredSig, TypeChecker, TypedCrate};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SealDriftError {
    #[error(
        "[E0080] sealed signature drift for `{name}`\n  lockfile: {locked}\n  current:  {current}"
    )]
    Drift {
        name: String,
        locked: String,
        current: String,
    },
    #[error("[E0080] sealed API item `{name}` missing from crisp.lock")]
    MissingInLock { name: String },
    #[error("[E0080] crisp.lock contains stale entry `{name}` not in current pub API")]
    StaleInLock { name: String },
    #[error("{0}")]
    Analysis(#[from] anyhow::Error),
}

pub fn compute_sealed_api(crate_root: &Path) -> Result<Vec<SealedSignature>, anyhow::Error> {
    let graph = load_module_graph(crate_root)?;
    let typed = TypeChecker::check_crate(crate_root)?;
    let ownership = OwnershipPass::analyze_crate(crate_root)?;
    let regions = RegionPass::assign_crate(crate_root)?;
    let errors = ErrorPass::analyze_crate(crate_root)?;

    let mut sigs = Vec::new();
    for (module_path, node) in &graph.modules {
        for sym in collect_module_symbols(module_path, &node.ast.items) {
            if sym.visibility != Visibility::Public || sym.from_prelude {
                continue;
            }
            let key = format!("{}::{}", sym.key.module, sym.key.name);
            let name = format!("{}::{}", sym.key.module, sym.key.name);

            let rust_signature = match sym.kind {
                crisp_resolve::symbols::SymbolKind::Function => {
                    if let (Some(o), Some(t), Some(e)) = (
                        ownership.signatures.get(&key),
                        typed.signatures.get(&key),
                        errors.signatures.get(&key),
                    ) {
                        format_fn_sealed(o, t, e, regions.lifetimes.get(&key))
                    } else {
                        continue;
                    }
                }
                crisp_resolve::symbols::SymbolKind::Type => {
                    format_type_sealed(&node.ast.items, &sym.key.name, &typed)
                }
                _ => continue,
            };

            sigs.push(SealedSignature {
                name,
                rust_signature,
            });
        }
    }

    sigs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(sigs)
}

fn format_fn_sealed(
    ownership: &OwnershipSignature,
    typed: &InferredSig,
    errors: &ErrorSig,
    lifetime: Option<&crisp_regions::LifetimeSig>,
) -> String {
    let mut line = format_owned_sig(ownership, Some(typed));
    if let Some(first) = line.lines().next() {
        line = first.to_string();
    }
    if let Some(lt) = lifetime {
        if !lt.elided && !lt.lifetime_params.is_empty() {
            line = format!("<{}> {}", lt.lifetime_params.join(", "), line);
        }
    }
    if errors.fallible && !errors.errors.is_empty() {
        let err_set = errors
            .errors
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        line = format!("{line} ! {err_set}");
    }
    if ownership.name == "main" {
        line = format!("pub {line}");
    }
    line
}

fn format_type_sealed(items: &[Item], type_name: &str, typed: &TypedCrate) -> String {
    for item in items {
        if let Item::TypeDef(td) = item {
            if td.name.name != type_name {
                continue;
            }
            return match &td.body {
                TypeBody::Struct(fields) => {
                    let field_strs: Vec<_> = fields
                        .iter()
                        .map(|f| format!("{}: {}", f.name.name, ast_type_name(&f.ty)))
                        .collect();
                    format!("pub struct {type_name} {{ {} }}", field_strs.join(", "))
                }
                TypeBody::Enum(_) => format!("pub enum {type_name} {{ .. }}"),
                TypeBody::Alias(ty) => format!("pub type {type_name} = {}", ast_type_name(ty)),
            };
        }
    }
    let _ = typed;
    format!("pub type {type_name}")
}

fn ast_type_name(ty: &crisp_ast::ty::Type) -> String {
    use crisp_ast::ty::TypeKind;
    match &ty.kind {
        TypeKind::Named(id) => id.name.clone(),
        _ => "_".into(),
    }
}

pub fn format_sealed_api(sigs: &[SealedSignature]) -> String {
    sigs.iter()
        .map(|s| format!("{}: {}", s.name, s.rust_signature))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn verify_sealed_api(crate_root: &Path) -> Result<(), SealDriftError> {
    let lock = match read_lock(crate_root).map_err(|e| SealDriftError::Analysis(e.into()))? {
        Some(l) => l,
        None => return Ok(()),
    };

    let current = compute_sealed_api(crate_root).map_err(SealDriftError::Analysis)?;
    let locked: BTreeMap<_, _> = lock
        .sealed_api
        .iter()
        .map(|s| (s.name.as_str(), s.rust_signature.as_str()))
        .collect();
    let now: BTreeMap<_, _> = current
        .iter()
        .map(|s| (s.name.as_str(), s.rust_signature.as_str()))
        .collect();

    for (name, sig) in &now {
        match locked.get(name) {
            Some(locked_sig) if *locked_sig != *sig => {
                return Err(SealDriftError::Drift {
                    name: (*name).to_string(),
                    locked: (*locked_sig).to_string(),
                    current: (*sig).to_string(),
                });
            }
            None => {
                return Err(SealDriftError::MissingInLock {
                    name: (*name).to_string(),
                });
            }
            _ => {}
        }
    }

    for name in locked.keys() {
        if !now.contains_key(name) {
            return Err(SealDriftError::StaleInLock {
                name: (*name).to_string(),
            });
        }
    }

    Ok(())
}

pub fn update_lock(crate_root: &Path) -> Result<CrispLock, anyhow::Error> {
    let manifest = read_manifest(crate_root)?;
    let sealed_api = compute_sealed_api(crate_root)?;
    let dependencies = resolve_dependencies(&manifest);
    let lock = CrispLock {
        version: LOCK_VERSION,
        dependencies,
        sealed_api,
    };
    write_lock(crate_root, &lock)?;
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crisp_manifest::read_lock;
    use std::path::PathBuf;

    fn hello_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
    }

    #[test]
    fn compute_hello_sealed_api() {
        let sigs = compute_sealed_api(&hello_root()).expect("seal");
        assert!(sigs.iter().any(|s| s.name == "main::main"));
    }

    #[test]
    fn update_and_verify_lock() {
        let dir = tempfile::TempDir::new().unwrap();
        copy_crate(&hello_root(), dir.path());
        let lock = update_lock(dir.path()).expect("update");
        assert!(!lock.sealed_api.is_empty());
        assert!(lock.dependencies.iter().any(|d| d.name == "tokio"));
        verify_sealed_api(dir.path()).expect("verify ok");
        let read = read_lock(dir.path()).unwrap().expect("file");
        assert_eq!(read.sealed_api, lock.sealed_api);
    }

    #[test]
    fn drift_detected_when_signature_changes() {
        let dir = tempfile::TempDir::new().unwrap();
        copy_crate(&hello_root(), dir.path());
        update_lock(dir.path()).unwrap();

        let lock_path = dir.path().join("crisp.lock");
        let mut lock: CrispLock =
            serde_json::from_str(&std::fs::read_to_string(&lock_path).unwrap()).unwrap();
        if let Some(sig) = lock.sealed_api.iter_mut().find(|s| s.name == "main::main") {
            sig.rust_signature = "pub fn main() -> DRIFT".into();
        }
        std::fs::write(&lock_path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();

        let err = verify_sealed_api(dir.path()).unwrap_err();
        assert!(matches!(err, SealDriftError::Drift { .. }));
    }

    fn copy_crate(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let dest = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_crate(&entry.path(), &dest);
            } else {
                std::fs::copy(entry.path(), dest).unwrap();
            }
        }
    }
}
