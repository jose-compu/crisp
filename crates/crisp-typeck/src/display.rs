use crate::types::{InferredSig, Ty};

pub fn format_sig(sig: &InferredSig) -> String {
    let params = sig
        .params
        .iter()
        .map(|(n, t)| format!("{n}: {}", format_ty(t)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({}) -> {}", sig.name, params, format_ty(&sig.ret))
}

pub fn format_ty(ty: &Ty) -> String {
    match ty {
        Ty::Never => "Never".into(),
        Ty::Unit => "()".into(),
        Ty::Bool => "bool".into(),
        Ty::Int => "int".into(),
        Ty::UInt => "uint".into(),
        Ty::Float => "float".into(),
        Ty::Char => "char".into(),
        Ty::Str => "str".into(),
        Ty::StrSlice => "&str".into(),
        Ty::Var(v) => format!("?{v}"),
        Ty::Tuple(ts) => format!(
            "({})",
            ts.iter().map(format_ty).collect::<Vec<_>>().join(", ")
        ),
        Ty::Array { elem, len } => format!("[{}; {len}]", format_ty(elem)),
        Ty::Slice(inner) => format!("[{}]", format_ty(inner)),
        Ty::Fn { params, ret } => format!(
            "({}) -> {}",
            params.iter().map(format_ty).collect::<Vec<_>>().join(", "),
            format_ty(ret)
        ),
        Ty::Option(inner) => format!("?{}", format_ty(inner)),
        Ty::Ref { mutable, inner } => {
            if *mutable {
                format!("&mut {}", format_ty(inner))
            } else {
                format!("&{}", format_ty(inner))
            }
        }
        Ty::Named { name, args } if args.is_empty() => name.clone(),
        Ty::Named { name, args } => format!(
            "{name}<{}>",
            args.iter().map(format_ty).collect::<Vec<_>>().join(", ")
        ),
        Ty::Error => "_".into(),
    }
}
