use crate::result::{CrispErrorEnum, ErrorResult, ErrorSig};
use crisp_typeck::{InferredSig, format_ty};

pub fn format_error_sig(sig: &ErrorSig, typed: Option<&InferredSig>) -> String {
    let ret = typed
        .map(|t| format_ty(&t.ret))
        .unwrap_or_else(|| "()".into());
    if !sig.fallible {
        if let Some(ts) = typed {
            let params = ts
                .params
                .iter()
                .map(|(n, t)| format!("{n}: {}", format_ty(t)))
                .collect::<Vec<_>>()
                .join(", ");
            return format!("{}({params}) -> {ret}", sig.name);
        }
        return format!("{}() -> {ret}", sig.name);
    }

    let params = typed
        .map(|t| {
            t.params
                .iter()
                .map(|(n, ty)| format!("{n}: {}", format_ty(ty)))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let err_set = sig.errors.iter().cloned().collect::<Vec<_>>().join(" | ");

    if params.is_empty() {
        format!("{}() -> {ret} ! {err_set}", sig.name)
    } else {
        format!("{}({params}) -> {ret} ! {err_set}", sig.name)
    }
}

pub fn format_crisp_error_enum(en: &CrispErrorEnum) -> String {
    if en.variants.is_empty() {
        return "// CrispError: (no fallible functions)".into();
    }
    let mut lines = vec!["enum CrispError {".to_string()];
    for v in &en.variants {
        if v.name == "Thrown" {
            lines.push("    Thrown(String),".into());
        } else {
            lines.push(format!("    {}({}),", v.name, v.payload_type));
        }
    }
    lines.push("}".into());
    lines.join("\n")
}

pub fn format_errors_crate(result: &ErrorResult, typed: &crisp_typeck::TypedCrate) -> String {
    let mut lines: Vec<String> = result
        .signatures
        .values()
        .map(|sig| {
            let key = format!("{}::{}", sig.module, sig.name);
            format_error_sig(sig, typed.signatures.get(&key))
        })
        .collect();
    lines.sort();
    let mut out = lines.join("\n");
    if !result.crisp_error.variants.is_empty() {
        out.push_str("\n\n");
        out.push_str(&format_crisp_error_enum(&result.crisp_error));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorPass;
    use crisp_typeck::TypeChecker;
    use std::path::PathBuf;

    #[test]
    fn format_fallible_read_config() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fallible");
        let typed = TypeChecker::check_crate(&root).unwrap();
        let errors = ErrorPass::analyze_crate(&root).unwrap();
        let out = format_errors_crate(&errors, &typed);
        assert!(out.contains("read_config"));
        assert!(out.contains("IoError"));
        assert!(out.contains("enum CrispError"));
    }
}
