use crate::lattice::OwnershipMode;
use crate::result::{OwnershipResult, OwnershipSignature};
use crisp_typeck::{InferredSig, format_ty};

pub fn format_owned_sig(sig: &OwnershipSignature, typed: Option<&InferredSig>) -> String {
    if let Some(ts) = typed {
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, (n, mode))| {
                let ty = ts.params.get(i).map(|(_, t)| t);
                match (ty, mode) {
                    (Some(t), OwnershipMode::Borrow) if t.is_stringish() => {
                        format!("{n}: &str")
                    }
                    (Some(t), OwnershipMode::Borrow) => format!("{n}: &{}", format_ty(t)),
                    (Some(t), OwnershipMode::MutBorrow) => {
                        format!("{n}: &mut {}", format_ty(t))
                    }
                    (Some(t), OwnershipMode::Owned) => format!("{n}: {}", format_ty(t)),
                    (_, mode) => format!("{n}: {}", mode.display()),
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let ret = format_ty(&ts.ret);
        let mut line = format!("{}({params}) -> {ret}", sig.name);
        for ac in &sig.auto_clones {
            line.push('\n');
            line.push_str(&ac.note);
        }
        return line;
    }
    let params = sig
        .params
        .iter()
        .map(|(n, mode)| format!("{n}: {}", mode.display()))
        .collect::<Vec<_>>()
        .join(", ");
    let mut line = format!("{}({params})", sig.name);
    for ac in &sig.auto_clones {
        line.push('\n');
        line.push_str(&ac.note);
    }
    line
}

pub fn format_ownership_crate(result: &OwnershipResult, typed: &crisp_typeck::TypedCrate) -> String {
    let mut lines: Vec<String> = result
        .signatures
        .values()
        .map(|sig| {
            let key = format!("{}::{}", sig.module, sig.name);
            format_owned_sig(sig, typed.signatures.get(&key))
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OwnershipPass;
    use crisp_typeck::TypeChecker;
    use std::path::PathBuf;

    #[test]
    fn format_hello_greet() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
        let typed = TypeChecker::check_crate(&root).unwrap();
        let ownership = OwnershipPass::analyze_crate(&root).unwrap();
        let out = format_ownership_crate(&ownership, &typed);
        assert!(out.contains("greet(name: &str)"));
    }
}
