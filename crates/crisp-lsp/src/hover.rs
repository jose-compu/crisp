//! Hover signatures, types, ownership, reachable errors.

use crate::walk::{Located, locate_at_offset};
use crisp_ast::item::SourceFile;
use crisp_errors::{ErrorResult, format_error_sig};
use crisp_ownership::{OwnershipResult, format_owned_sig};
use crisp_regions::{RegionResult, format_lifetime_sig};
use crisp_typeck::{TypedCrate, format_sig, format_ty};

#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub title: String,
    pub detail: Option<String>,
    pub markdown: String,
}

pub fn hover_at_offset(
    file: &SourceFile,
    module: &str,
    offset: u32,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    regions: &RegionResult,
    errors: &ErrorResult,
) -> Option<HoverInfo> {
    let loc = locate_at_offset(file, offset)?;
    match loc {
        Located::Call { callee, .. } => {
            hover_call(module, callee, typed, ownership, regions, errors)
        }
        Located::Ident(id) => hover_ident(module, &id.name, typed, ownership, regions, errors),
        Located::Function(f) => hover_function(module, f, typed, ownership, regions, errors),
        Located::Expr(_) => None,
    }
}

fn hover_call(
    module: &str,
    callee: &crisp_ast::ident::Ident,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    regions: &RegionResult,
    errors: &ErrorResult,
) -> Option<HoverInfo> {
    let key = format!("{module}::{}", callee.name);
    let mut parts = Vec::new();
    if let Some(sig) = typed.signatures.get(&key) {
        parts.push(format!("**type** `{}`", format_sig(sig)));
    }
    if let Some(osig) = ownership.signatures.get(&key) {
        parts.push(format!(
            "**ownership** `{}`",
            format_owned_sig(osig, typed.signatures.get(&key))
        ));
    }
    if let Some(lsig) = regions.lifetimes.get(&key) {
        parts.push(format!(
            "**lifetimes** `{}`",
            format_lifetime_sig(lsig, typed.signatures.get(&key))
        ));
    }
    if let Some(esig) = errors.signatures.get(&key) {
        parts.push(format!(
            "**errors** `{}`",
            format_error_sig(esig, typed.signatures.get(&key))
        ));
    } else if let Some(esig) = errors.signatures.get(&callee.name) {
        parts.push(format!(
            "**errors** `{}`",
            format_error_sig(esig, typed.signatures.get(&key))
        ));
    }
    if parts.is_empty() {
        return None;
    }
    let title = callee.name.clone();
    let markdown = parts.join("\n\n");
    Some(HoverInfo {
        title,
        detail: parts.first().cloned(),
        markdown,
    })
}

fn hover_ident(
    module: &str,
    name: &str,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    regions: &RegionResult,
    errors: &ErrorResult,
) -> Option<HoverInfo> {
    let key = format!("{module}::{name}");
    if let Some(sig) = typed.signatures.get(&key) {
        return hover_call(
            module,
            &crisp_ast::ident::Ident {
                name: name.to_string(),
                span: sig.span,
            },
            typed,
            ownership,
            regions,
            errors,
        );
    }
    for sig in typed.signatures.values() {
        for (pname, pty) in &sig.params {
            if pname == name {
                let mode = ownership
                    .signatures
                    .get(&format!("{}::{}", sig.module, sig.name))
                    .and_then(|o| {
                        o.params
                            .iter()
                            .find(|(n, _)| n == pname)
                            .map(|(_, m)| m.display().to_string())
                    })
                    .unwrap_or_else(|| "?".into());
                return Some(HoverInfo {
                    title: name.to_string(),
                    detail: Some(format!("{name}: {}", format_ty(pty))),
                    markdown: format!("**binding** `{name}: {}` ({mode})", format_ty(pty)),
                });
            }
        }
    }
    None
}

fn hover_function(
    module: &str,
    f: &crisp_ast::item::FunctionDef,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    regions: &RegionResult,
    errors: &ErrorResult,
) -> Option<HoverInfo> {
    hover_call(module, &f.name, typed, ownership, regions, errors)
}
