use anyhow::Result;
use crisp_manifest::read_lock;
use crisp_resolve::find_crate_root;
use crisp_rust_emit::{compute_sealed_api, format_sealed_api};
use std::path::Path;

pub fn reveal_seal(crate_path: &Path) -> Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    if let Some(lock) = read_lock(&root)? {
        return Ok(format_sealed_api(&lock.sealed_api));
    }
    let api = compute_sealed_api(&root)?;
    Ok(format_sealed_api(&api))
}
