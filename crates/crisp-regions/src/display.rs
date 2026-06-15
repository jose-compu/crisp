use crate::lifetime::{LifetimeSig, RegionResult};
use crisp_typeck::{InferredSig, format_ty};

pub fn format_lifetime_sig(sig: &LifetimeSig, typed: Option<&InferredSig>) -> String {
    if sig.elided {
        if let Some(ts) = typed {
            let params = ts
                .params
                .iter()
                .map(|(n, t)| format!("{n}: {}", format_ty(t)))
                .collect::<Vec<_>>()
                .join(", ");
            return format!("{}({params}) -> {}  [elided]", sig.name, format_ty(&ts.ret));
        }
        return format!("{}()  [elided]", sig.name);
    }

    let lt_prefix = if sig.lifetime_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", sig.lifetime_params.join(", "))
    };

    if let Some(ts) = typed {
        let params = ts
            .params
            .iter()
            .enumerate()
            .map(|(i, (n, t))| {
                if let Some(lt) = sig.param_lifetimes.get(i).and_then(|x| x.as_ref()) {
                    format!("{n}: {lt} {}", format_ty(t))
                } else {
                    format!("{n}: {}", format_ty(t))
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let ret = match &sig.ret_lifetime {
            Some(lt) => format!("{lt} {}", format_ty(&ts.ret)),
            None => format_ty(&ts.ret),
        };
        format!("{}{}({params}) -> {ret}", sig.name, lt_prefix)
    } else {
        format!("{}{}()", sig.name, lt_prefix)
    }
}

pub fn format_lifetimes_crate(result: &RegionResult, typed: &crisp_typeck::TypedCrate) -> String {
    let mut lines: Vec<String> = result
        .lifetimes
        .values()
        .map(|sig| {
            let key = format!("{}::{}", sig.module, sig.name);
            format_lifetime_sig(sig, typed.signatures.get(&key))
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegionPass;
    use crisp_typeck::TypeChecker;
    use std::path::PathBuf;

    #[test]
    fn format_hello() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
        let typed = TypeChecker::check_crate(&root).unwrap();
        let regions = RegionPass::assign_crate(&root).unwrap();
        let out = format_lifetimes_crate(&regions, &typed);
        assert!(out.contains("greet"));
    }
}
