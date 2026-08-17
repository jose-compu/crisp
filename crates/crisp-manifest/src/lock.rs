//! `crisp.lock` — resolved deps + sealed pub API (spec §12.5).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

pub const LOCK_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedSignature {
    pub name: String,
    pub rust_signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub rust: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Path as written in `crisp.toml` (relative to the Crisp crate root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrispLock {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<ResolvedDependency>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sealed_api: Vec<SealedSignature>,
}

impl Default for CrispLock {
    fn default() -> Self {
        Self {
            version: LOCK_VERSION,
            dependencies: Vec::new(),
            sealed_api: Vec::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum LockError {
    #[error("failed to read crisp.lock: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse crisp.lock: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported crisp.lock version {0} (expected {LOCK_VERSION})")]
    UnsupportedVersion(u32),
}

pub fn read_lock(crate_root: &Path) -> Result<Option<CrispLock>, LockError> {
    let path = crate_root.join("crisp.lock");
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    let lock: CrispLock = serde_json::from_str(&raw)?;
    if lock.version != LOCK_VERSION {
        return Err(LockError::UnsupportedVersion(lock.version));
    }
    Ok(Some(lock))
}

pub fn write_lock(crate_root: &Path, lock: &CrispLock) -> Result<(), LockError> {
    let path = crate_root.join("crisp.lock");
    let raw = serde_json::to_string_pretty(lock)?;
    fs::write(path, raw)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn roundtrip_lock() {
        let dir = TempDir::new().unwrap();
        let lock = CrispLock {
            version: LOCK_VERSION,
            dependencies: vec![ResolvedDependency {
                name: "tokio".into(),
                version: "1".into(),
                rust: true,
                features: vec!["rt".into(), "macros".into()],
                path: None,
            }],
            sealed_api: vec![SealedSignature {
                name: "main::main".into(),
                rust_signature: "pub fn main() -> ()".into(),
            }],
        };
        write_lock(dir.path(), &lock).unwrap();
        let read = read_lock(dir.path()).unwrap().expect("lock");
        assert_eq!(read, lock);
    }
}
