use crisp_ownership::{OwnershipPass, format_ownership_crate};
use crisp_resolve::find_crate_root;
use crisp_typeck::TypeChecker;
use std::path::Path;

pub fn reveal_ownership(crate_path: &Path) -> anyhow::Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let typed = TypeChecker::check_crate(&root)?;
    let ownership = OwnershipPass::analyze_crate(&root)?;
    Ok(format_ownership_crate(&ownership, &typed))
}
