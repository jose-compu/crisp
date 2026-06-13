use crisp_resolve::find_crate_root;
use crisp_typeck::{TypeChecker, format_sig};
use std::path::Path;

pub fn reveal_types(crate_path: &Path) -> anyhow::Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let typed = TypeChecker::check_crate(&root)?;
    let mut lines: Vec<String> = typed.signatures.values().map(format_sig).collect();
    lines.sort();
    Ok(lines.join("\n"))
}
