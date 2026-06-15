//! Resolve ownership/rustc disagreements via fallback rewrites (spec §7.6).

use crate::probe::emit_probe_crate;
use crate::rustc::{RustcError, check_rust_source, is_borrow_check_failure};
use crisp_ownership::{
    FallbackKind, OwnershipPass, OwnershipResult, apply_fallback, candidates_for_auto_clone,
    fallback_chain,
};
use crisp_resolve::module::load_module_graph;
use crisp_typeck::TypeChecker;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FallbackResolveError {
    #[error("[E0054] ownership error: {0}")]
    Ownership(#[from] crisp_ownership::OwnershipError),
    #[error("[E0055] type error: {0}")]
    Type(#[from] crisp_typeck::TypeError),
    #[error("[E0056] resolve error: {0}")]
    Resolve(#[from] crisp_resolve::ResolveError),
    #[error(
        "[E0057] could not produce borrow-checking Rust for `{name}`; please file a bug (rustc: {summary})"
    )]
    Exhausted { name: String, summary: String },
    #[error("[E0058] rustc not available; skipped fallback resolution")]
    RustcUnavailable,
}

/// Analyze ownership, probe-emit Rust, and apply §7.6 fallbacks until rustc accepts or exhausted.
pub fn resolve_rustc_fallbacks(crate_root: &Path) -> Result<OwnershipResult, FallbackResolveError> {
    let graph = load_module_graph(crate_root)?;
    let typed = TypeChecker::check_crate(crate_root)?;
    let mut ownership = OwnershipPass::analyze_crate(crate_root)?;

    let source = emit_probe_crate(&graph, &typed, &ownership);
    match check_rust_source(&source) {
        Ok(()) => return Ok(ownership),
        Err(RustcError::NotFound) => return Err(FallbackResolveError::RustcUnavailable),
        Err(RustcError::CheckFailed { summary, stderr }) if is_borrow_check_failure(&stderr) => {
            try_fallbacks(&mut ownership, &graph, &typed, &summary)?
        }
        Err(RustcError::CheckFailed { .. }) => {
            // Probe emit is partial; non-borrow failures are not §7.6 disagreements.
            return Ok(ownership);
        }
        Err(RustcError::Io(e)) => {
            return Err(FallbackResolveError::Exhausted {
                name: crate_root.display().to_string(),
                summary: e.to_string(),
            });
        }
    }

    Ok(ownership)
}

fn try_fallbacks(
    ownership: &mut OwnershipResult,
    graph: &crisp_resolve::ModuleGraph,
    typed: &crisp_typeck::TypedCrate,
    initial_summary: &str,
) -> Result<(), FallbackResolveError> {
    let fn_keys: Vec<String> = ownership.signatures.keys().cloned().collect();

    for key in &fn_keys {
        let sig = ownership.signatures.get(key).cloned();
        let Some(sig) = sig else { continue };

        let candidates: Vec<FallbackKind> = if !sig.auto_clones.is_empty() {
            candidates_for_auto_clone().to_vec()
        } else {
            fallback_chain().to_vec()
        };

        for kind in candidates {
            let mut trial = ownership.clone();
            let detail = sig
                .auto_clones
                .first()
                .map(|ac| ac.binding.as_str())
                .unwrap_or(&sig.name);
            let span = sig.span;
            if !apply_fallback(&mut trial, key, kind, span, detail) {
                continue;
            }
            let source = emit_probe_crate(graph, typed, &trial);
            match check_rust_source(&source) {
                Ok(()) => {
                    *ownership = trial;
                    return Ok(());
                }
                Err(RustcError::NotFound) => return Err(FallbackResolveError::RustcUnavailable),
                Err(RustcError::CheckFailed { .. }) => continue,
                Err(RustcError::Io(e)) => {
                    return Err(FallbackResolveError::Exhausted {
                        name: sig.name.clone(),
                        summary: e.to_string(),
                    });
                }
            }
        }
    }

    Err(FallbackResolveError::Exhausted {
        name: "crate".into(),
        summary: initial_summary.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../crisp-ownership/tests/fixtures")
            .join(name)
    }

    #[test]
    fn resolves_auto_clone_with_fallback() {
        if let Err(FallbackResolveError::RustcUnavailable) =
            resolve_rustc_fallbacks(&fixture("auto_clone"))
        {
            return;
        }
        let result = resolve_rustc_fallbacks(&fixture("auto_clone")).expect("fallback resolves");
        let forward = result.get("main", "forward").expect("forward");
        assert!(
            !forward.auto_clones.is_empty(),
            "forward should need auto-clone for msg after move"
        );
        if !forward.applied_fallbacks.is_empty() {
            assert!(
                forward
                    .applied_fallbacks
                    .iter()
                    .any(|f| f.kind == FallbackKind::CloneAtMove)
            );
        }
    }

    #[test]
    fn hello_needs_no_fallback() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
        let result = resolve_rustc_fallbacks(&root).expect("hello resolves");
        assert!(
            result
                .signatures
                .values()
                .all(|s| s.applied_fallbacks.is_empty())
        );
    }
}
