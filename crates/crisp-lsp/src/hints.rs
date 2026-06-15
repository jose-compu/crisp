//! Ghost-text type and ownership hints on bindings.

use crate::walk::all_bindings;
use crisp_ast::item::SourceFile;
use crisp_ownership::OwnershipResult;
use crisp_typeck::{TypedCrate, format_ty};

#[derive(Debug, Clone)]
pub struct InlayHint {
    pub position: u32,
    pub label: String,
    pub kind: InlayHintKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    Type,
    Ownership,
}

pub fn inlay_hints_for_file(
    file: &SourceFile,
    module: &str,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    for item in &file.items {
        if let crisp_ast::item::Item::Function(f) = item {
            let key = format!("{module}::{}", f.name.name);
            if let Some(osig) = ownership.signatures.get(&key) {
                for (i, (pname, mode)) in osig.params.iter().enumerate() {
                    if let Some((_, pty)) = typed.signatures.get(&key).and_then(|s| s.params.get(i))
                    {
                        if let Some(param) = f.params.get(i) {
                            hints.push(InlayHint {
                                position: param.name.span.end,
                                label: format!(": {} {}", format_ty(pty), mode.display()),
                                kind: InlayHintKind::Ownership,
                            });
                        }
                    }
                }
            }
        }
    }
    for (span, name, value) in all_bindings(file) {
        if let Some(ty) = infer_binding_type(module, &name, &value, typed) {
            hints.push(InlayHint {
                position: span.end,
                label: format!(": {}", format_ty(&ty)),
                kind: InlayHintKind::Type,
            });
        }
    }
    hints.sort_by_key(|h| h.position);
    hints
}

fn infer_binding_type(
    module: &str,
    name: &str,
    value: &crisp_ast::expr::Expr,
    typed: &TypedCrate,
) -> Option<crisp_typeck::Ty> {
    let _ = (module, name);
    match &value.kind {
        crisp_ast::expr::ExprKind::Call { func, .. } => {
            if let crisp_ast::expr::ExprKind::Ident(id) = &func.kind {
                let key = format!("{module}::{}", id.name);
                return typed.signatures.get(&key).map(|s| s.ret.clone());
            }
        }
        crisp_ast::expr::ExprKind::Int(_) => return Some(crisp_typeck::Ty::Int),
        crisp_ast::expr::ExprKind::Str(_) => return Some(crisp_typeck::Ty::Str),
        _ => {}
    }
    None
}
