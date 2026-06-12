//! Sealed-crate signature lockfile (spec §12.5).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedSignature {
    pub name: String,
    pub rust_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrispLock {
    pub version: u32,
    pub sealed_api: Vec<SealedSignature>,
}
