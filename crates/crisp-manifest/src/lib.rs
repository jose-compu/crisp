//! `crisp.toml` / `crisp.lock` (spec §12.5, §18).

mod lock;
mod manifest;
mod resolve;

pub use lock::{
    CrispLock, LOCK_VERSION, LockError, ResolvedDependency, SealedSignature, read_lock, write_lock,
};
pub use manifest::{
    BuildSection, CrateManifest, DependencySpec, ManifestError, parse_manifest_str, read_manifest,
};
pub use resolve::resolve_dependencies;
