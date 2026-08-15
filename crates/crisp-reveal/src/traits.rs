use anyhow::Result;
use crisp_cir::CirBuilder;
use crisp_rust_emit::format_ty;
use std::fmt::Write;
use std::path::Path;

pub fn reveal_traits(crate_root: &Path) -> Result<String> {
    let cir = CirBuilder::build_crate(crate_root)?;
    let mut out = String::new();
    for m in &cir.modules {
        for item in &m.items {
            match item {
                crisp_cir::CirItem::Trait(t) => {
                    let gens = if t.generics.is_empty() {
                        String::new()
                    } else {
                        format!("<{}>", t.generics.join(", "))
                    };
                    let _ = writeln!(out, "trait {}{gens} {{", t.name);
                    for meth in &t.methods {
                        let params: Vec<_> = meth
                            .params
                            .iter()
                            .map(|(n, ty)| format!("{n}: {}", format_ty(ty)))
                            .collect();
                        let ret = if matches!(meth.ret, crisp_cir::CirTy::Unit) {
                            String::new()
                        } else {
                            format!(" -> {}", format_ty(&meth.ret))
                        };
                        let _ = writeln!(out, "    {}({}){ret}", meth.name, params.join(", "));
                    }
                    let _ = writeln!(out, "}}");
                }
                crisp_cir::CirItem::Impl(ib) => {
                    if let Some(tn) = &ib.trait_name {
                        let args = if ib.trait_args.is_empty() {
                            String::new()
                        } else {
                            format!(
                                "<{}>",
                                ib.trait_args
                                    .iter()
                                    .map(format_ty)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        };
                        let _ = writeln!(out, "impl {tn}{args} for {}", ib.ty_name);
                    }
                }
                _ => {}
            }
        }
    }
    for shape in &cir.shape_traits {
        let gens = if shape.generics.is_empty() {
            String::new()
        } else {
            format!("<{}>", shape.generics.join(", "))
        };
        let _ = writeln!(out, "shape {}{gens} {{", shape.name);
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
            let args = if imp.args.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    imp.args
                        .iter()
                        .map(format_ty)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            let _ = writeln!(out, "impl {}{args} for {}", shape.name, imp.ty_name);
        }
        let _ = writeln!(out);
    }
    if out.is_empty() {
        out.push_str("// (no traits in this crate)\n");
    }
    Ok(out)
}
