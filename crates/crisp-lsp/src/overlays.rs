//! Reachable CrispError set overlays on call sites.

use crate::walk::all_calls;
use crisp_ast::Span;
use crisp_ast::item::SourceFile;
use crisp_errors::{ErrorResult, format_error_sig};
use crisp_typeck::TypedCrate;

#[derive(Debug, Clone)]
pub struct CallOverlay {
    pub span: Span,
    pub callee: String,
    pub error_set: String,
    pub fallible: bool,
}

pub fn call_overlays_for_file(
    file: &SourceFile,
    module: &str,
    errors: &ErrorResult,
    typed: &TypedCrate,
) -> Vec<CallOverlay> {
    let mut out = Vec::new();
    for (span, callee, _) in all_calls(file) {
        let key = format!("{module}::{}", callee.name);
        let esig = errors
            .signatures
            .get(&key)
            .or_else(|| errors.signatures.get(&callee.name));
        if let Some(esig) = esig
            && esig.fallible
        {
            let error_set = esig.errors.iter().cloned().collect::<Vec<_>>().join(" | ");
            out.push(CallOverlay {
                span,
                callee: callee.name.clone(),
                error_set: if error_set.is_empty() {
                    format_error_sig(esig, typed.signatures.get(&key))
                } else {
                    error_set
                },
                fallible: true,
            });
        }
    }
    out
}
