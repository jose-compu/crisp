use anyhow::Result;
use crisp_cir::CirBuilder;
use crisp_rust_emit::format_ty;
use std::fmt::Write;
use std::path::Path;

pub fn reveal_traits(crate_root: &Path) -> Result<String> {
    let cir = CirBuilder::build_crate(crate_root)?;
    let mut out = String::new();
    for shape in &cir.shape_traits {
        let _ = writeln!(out, "shape {} {{", shape.name);
        for (name, ty) in &shape.fields {
            let _ = writeln!(out, "    {name}: {}", format_ty(ty));
        }
        for m in &shape.methods {
            let params: Vec<_> = m
                .params
                .iter()
                .map(|(n, t)| format!("{n}: {}", format_ty(t)))
                .collect();
            let _ = writeln!(
                out,
                "    fn {}({}) -> {}",
                m.name,
                params.join(", "),
                format_ty(&m.ret)
            );
        }
        let _ = writeln!(out, "}}");
        for imp in &shape.impls {
            let _ = writeln!(out, "impl {} for {}", shape.name, imp.ty_name);
        }
        let _ = writeln!(out);
    }
    if out.is_empty() {
        out.push_str("// (no shape traits in this crate)\n");
    }
    Ok(out)
}
