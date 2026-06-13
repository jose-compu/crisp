use crate::lattice::OwnershipMode;
use crate::result::{AppliedFallback, FallbackKind, OwnershipResult};
use crisp_ast::Span;

/// Ordered fallback chain per spec §7.6.
pub fn fallback_chain() -> &'static [FallbackKind] {
    &[
        FallbackKind::Reborrow,
        FallbackKind::CloneAtMove,
        FallbackKind::WidenMut,
    ]
}

/// Candidates for a function that already has detected auto-clone sites.
pub fn candidates_for_auto_clone() -> &'static [FallbackKind] {
    &[FallbackKind::CloneAtMove, FallbackKind::Reborrow]
}

pub fn apply_fallback(
    result: &mut OwnershipResult,
    fn_key: &str,
    kind: FallbackKind,
    span: Span,
    detail: &str,
) -> bool {
    let Some(sig) = result.signatures.get_mut(fn_key) else {
        return false;
    };

    match kind {
        FallbackKind::Reborrow => {
            let mut changed = false;
            for (_, mode) in &mut sig.params {
                if *mode == OwnershipMode::Owned {
                    *mode = OwnershipMode::Borrow;
                    changed = true;
                }
            }
            if !changed {
                return false;
            }
        }
        FallbackKind::WidenMut => {
            let mut changed = false;
            for (_, mode) in &mut sig.params {
                if *mode == OwnershipMode::Borrow {
                    *mode = OwnershipMode::MutBorrow;
                    changed = true;
                }
            }
            if !changed {
                return false;
            }
        }
        FallbackKind::CloneAtMove => {
            if sig.auto_clones.is_empty() {
                return false;
            }
        }
    }

    let note = format!(
        "[rustc-fallback: {} @ offset {}] {detail}",
        kind.label(),
        span.start
    );
    sig.applied_fallbacks.push(AppliedFallback {
        kind,
        span,
        note,
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::{AutoClone, OwnershipSignature};
    use std::collections::BTreeMap;

    fn sample_result() -> OwnershipResult {
        let mut signatures = BTreeMap::new();
        signatures.insert(
            "main::forward".into(),
            OwnershipSignature {
                module: "main".into(),
                name: "forward".into(),
                params: vec![("msg".into(), OwnershipMode::Borrow)],
                ret_mode: None,
                auto_clones: vec![AutoClone {
                    binding: "msg".into(),
                    span: Span::new(0, 1),
                    note: "[auto-clone] msg".into(),
                }],
                applied_fallbacks: vec![],
                span: Span::new(0, 10),
            },
        );
        OwnershipResult { signatures }
    }

    #[test]
    fn clone_fallback_applies_when_auto_clone_present() {
        let mut result = sample_result();
        assert!(apply_fallback(
            &mut result,
            "main::forward",
            FallbackKind::CloneAtMove,
            Span::new(5, 6),
            "msg"
        ));
        assert_eq!(result.signatures["main::forward"].applied_fallbacks.len(), 1);
    }

    #[test]
    fn clone_fallback_skipped_without_auto_clone() {
        let mut result = sample_result();
        result.signatures.get_mut("main::forward").unwrap().auto_clones.clear();
        assert!(!apply_fallback(
            &mut result,
            "main::forward",
            FallbackKind::CloneAtMove,
            Span::new(0, 1),
            "msg"
        ));
    }

    #[test]
    fn reborrow_widens_owned_params() {
        let mut result = sample_result();
        result.signatures.get_mut("main::forward").unwrap().params[0].1 =
            OwnershipMode::Owned;
        assert!(apply_fallback(
            &mut result,
            "main::forward",
            FallbackKind::Reborrow,
            Span::new(0, 1),
            "msg"
        ));
        assert_eq!(
            result.signatures["main::forward"].params[0].1,
            OwnershipMode::Borrow
        );
    }
}
