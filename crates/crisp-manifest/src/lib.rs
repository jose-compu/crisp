//! `crisp.toml` / `crisp.lock` (spec §12.5, §18).

mod lock;
mod manifest;
mod resolve;

pub use lock::{CrispLock, LockError, ResolvedDependency, SealedSignature, read_lock, write_lock, LOCK_VERSION};
pub use manifest::{BuildSection, CrateManifest, DependencySpec, ManifestError, read_manifest, parse_manifest_str};
pub use resolve::resolve_dependencies;
