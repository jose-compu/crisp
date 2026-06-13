use anyhow::Result;
use crisp_resolve::find_crate_root;
use std::fmt::Write;
use std::path::Path;

pub fn reveal_diff(crate_path: &Path) -> Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let graph = crisp_resolve::module::load_module_graph(&root)?;
    let rust = crate::rust::reveal_rust(&root)?;
    let mut crisp_src = String::new();
    for node in graph.modules.values() {
        let _ = writeln!(crisp_src, "-- {}", node.module_path);
        for item in &node.ast.items {
            if let crisp_ast::item::Item::Function(f) = item {
                let _ = writeln!(crisp_src, "fn {}", f.name.name);
            }
        }
    }
    let mut out = String::from("=== Crisp (summary) ===\n");
    out.push_str(&crisp_src);
    out.push_str("\n=== Emitted Rust ===\n");
    out.push_str(&rust);
    Ok(out)
}
