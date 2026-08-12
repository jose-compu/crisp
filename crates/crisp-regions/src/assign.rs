use crate::lifetime::{LifetimeSig, RegionResult};
use crisp_ast::item::{FunctionDef, Item};
use crisp_ownership::{OwnershipMode, OwnershipPass};
use crisp_resolve::module::load_module_graph;
use crisp_typeck::{InferredSig, Ty, TypeChecker};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegionError {
    #[error("[E0060] region analysis failed: {message}")]
    Internal { message: String },
    #[error("[E0061] ownership error: {0}")]
    Ownership(#[from] crisp_ownership::OwnershipError),
    #[error("[E0062] type error: {0}")]
    Type(#[from] crisp_typeck::TypeError),
}

pub struct RegionPass;

impl RegionPass {
    pub fn assign_crate(crate_root: &Path) -> Result<RegionResult, RegionError> {
        let ownership = OwnershipPass::analyze_crate(crate_root)?;
        let typed = TypeChecker::check_crate(crate_root)?;
        let graph = load_module_graph(crate_root).map_err(|e| RegionError::Internal {
            message: e.to_string(),
        })?;

        let mut fn_defs: BTreeMap<String, (String, FunctionDef)> = BTreeMap::new();
        for node in graph.modules.values() {
            for item in &node.ast.items {
                match item {
                    Item::Function(f) => {
                        let key = format!("{}::{}", node.module_path, f.name.name);
                        fn_defs.insert(key, (node.module_path.clone(), f.clone()));
                    }
                    Item::Impl(ib) => {
                        let ty_name = match &ib.ty.kind {
                            crisp_ast::ty::TypeKind::Named(id) => id.name.clone(),
                            _ => continue,
                        };
                        for f in &ib.items {
                            let key = format!("{}::{ty_name}::{}", node.module_path, f.name.name);
                            fn_defs.insert(key, (node.module_path.clone(), f.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut lifetimes = BTreeMap::new();
        for (key, (module, def)) in &fn_defs {
            let osig = ownership.signatures.get(key);
            let tsig = typed.signatures.get(key);
            lifetimes.insert(
                key.clone(),
                assign_function_lifetimes(module, def, osig, tsig),
            );
        }

        Ok(RegionResult { lifetimes })
    }
}

fn assign_function_lifetimes(
    module: &str,
    def: &FunctionDef,
    ownership: Option<&crisp_ownership::OwnershipSignature>,
    typed: Option<&InferredSig>,
) -> LifetimeSig {
    let _ = module;
    let mut explicit: BTreeMap<String, String> = BTreeMap::new();
    for p in &def.params {
        if let Some(lt) = &p.lifetime {
            explicit.insert(p.name.name.clone(), lt.name.clone());
        }
    }

    let ref_param_count = count_ref_params(ownership, typed);
    let ret_is_ref = typed
        .map(|s| matches!(s.ret, Ty::Ref { .. }))
        .unwrap_or(false);

    let mut param_lifetimes: Vec<Option<String>> = Vec::new();
    let mut lifetime_params: Vec<String> = Vec::new();

    if explicit.is_empty() {
        if ret_is_ref && ref_param_count >= 2 {
            lifetime_params.push("'a".into());
            if let Some(ts) = typed {
                for (i, (name, ty)) in ts.params.iter().enumerate() {
                    let mode = ownership
                        .and_then(|o| o.params.get(i).map(|(_, m)| *m))
                        .unwrap_or(OwnershipMode::Borrow);
                    if is_ref_param(ty, mode) {
                        param_lifetimes.push(Some("'a".into()));
                    } else {
                        param_lifetimes.push(None);
                    }
                    let _ = name;
                }
            }
        } else if ret_is_ref && ref_param_count == 1 {
            // Rust elision — no explicit lifetime emitted
            if let Some(ts) = typed {
                for _ in ts.params.iter() {
                    param_lifetimes.push(None);
                }
            }
        } else {
            if let Some(ts) = typed {
                for _ in ts.params.iter() {
                    param_lifetimes.push(None);
                }
            }
        }
    } else {
        let mut regions: BTreeMap<String, String> = BTreeMap::new();
        for p in &def.params {
            if let Some(lt) = &p.lifetime {
                regions.entry(lt.name.clone()).or_insert_with(|| {
                    let name = format!("'{}", lt.name);
                    if !lifetime_params.contains(&name) {
                        lifetime_params.push(name.clone());
                    }
                    name
                });
                param_lifetimes.push(regions.get(&lt.name).cloned());
            } else {
                param_lifetimes.push(None);
            }
        }
    }

    let ret_lifetime = if ret_is_ref {
        if lifetime_params.len() == 1 && ref_param_count >= 2 {
            Some(lifetime_params[0].clone())
        } else {
            None
        }
    } else {
        None
    };

    let elided = ret_is_ref && ref_param_count <= 1 && explicit.is_empty();

    LifetimeSig {
        module: module.to_string(),
        name: def.name.name.clone(),
        lifetime_params,
        param_lifetimes,
        ret_lifetime,
        elided,
    }
}

fn count_ref_params(
    ownership: Option<&crisp_ownership::OwnershipSignature>,
    typed: Option<&InferredSig>,
) -> usize {
    match (ownership, typed) {
        (Some(o), Some(t)) => o
            .params
            .iter()
            .zip(t.params.iter())
            .filter(|((_, mode), (_, ty))| is_ref_param(ty, *mode))
            .count(),
        _ => 0,
    }
}

fn is_ref_param(ty: &Ty, mode: OwnershipMode) -> bool {
    matches!(mode, OwnershipMode::Borrow | OwnershipMode::MutBorrow)
        || matches!(ty, Ty::Ref { .. } | Ty::StrSlice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn examples(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
    }

    #[test]
    fn hello_lifetimes_elided() {
        let result = RegionPass::assign_crate(&examples("hello")).expect("regions hello");
        let greet = result.get("main", "greet").expect("greet");
        assert!(greet.elided || greet.lifetime_params.is_empty());
    }

    #[test]
    fn longest_two_refs_unified() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/longest");
        let result = RegionPass::assign_crate(&root).expect("regions longest");
        let longest = result.get("main", "longest").expect("longest fn");
        assert!(longest.lifetime_params.iter().any(|l| l.contains('a')));
    }
}
