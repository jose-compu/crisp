use crisp_regions::{RegionPass, format_lifetimes_crate};
use crisp_resolve::find_crate_root;
use crisp_typeck::TypeChecker;
use std::path::Path;

pub fn reveal_lifetimes(crate_path: &Path) -> anyhow::Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let typed = TypeChecker::check_crate(&root)?;
    let regions = RegionPass::assign_crate(&root)?;
    Ok(format_lifetimes_crate(&regions, &typed))
}
