use anyhow::Result;
use crisp_rust_emit::emit_to_target;
use std::path::Path;

pub fn reveal_rust(crate_root: &Path) -> Result<String> {
    let out = emit_to_target(crate_root)?;
    Ok(out.main_rs)
}
