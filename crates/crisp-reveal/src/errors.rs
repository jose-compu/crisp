use crisp_errors::{ErrorPass, format_errors_crate};
use crisp_resolve::find_crate_root;
use crisp_typeck::TypeChecker;
use std::path::Path;

pub fn reveal_errors(crate_path: &Path) -> anyhow::Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let typed = TypeChecker::check_crate(&root)?;
    let errors = ErrorPass::analyze_crate(&root)?;
    Ok(format_errors_crate(&errors, &typed))
}
