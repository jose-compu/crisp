use crisp_ownership::OwnershipPass;
use crisp_rust_emit::resolve_rustc_fallbacks;
use crisp_resolve::find_crate_root;
use crisp_typeck::TypeChecker;
use std::path::Path;

pub fn reveal_ownership(crate_path: &Path) -> anyhow::Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let typed = TypeChecker::check_crate(&root)?;
    let ownership = match resolve_rustc_fallbacks(&root) {
        Ok(o) => o,
        Err(crisp_rust_emit::FallbackResolveError::RustcUnavailable) => {
            OwnershipPass::analyze_crate(&root)?
        }
        Err(e) => return Err(e.into()),
    };
    Ok(crisp_ownership::format_ownership_crate(&ownership, &typed))
}
