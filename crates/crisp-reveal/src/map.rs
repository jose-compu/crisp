use anyhow::Result;
use crisp_cir::CirBuilder;
use crisp_resolve::find_crate_root;
use std::fmt::Write;
use std::path::Path;

pub fn reveal_map(crate_path: &Path) -> Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let cir = CirBuilder::build_crate(&root)?;
    let mut out = String::from("-- drop/alloc map (inherited from emitted Rust Drop)\n");
    for m in &cir.modules {
        for item in &m.items {
            match item {
                crisp_cir::CirItem::Struct(s) => {
                    let _ = writeln!(
                        out,
                        "{}::{} — stack struct; heap fields follow Rust layout",
                        m.path, s.name
                    );
                }
                crisp_cir::CirItem::Function(f) => {
                    let _ = writeln!(
                        out,
                        "{}::{} — locals dropped at end of block (Rust scope rules)",
                        m.path, f.name
                    );
                }
                _ => {}
            }
        }
    }
    Ok(out)
}
