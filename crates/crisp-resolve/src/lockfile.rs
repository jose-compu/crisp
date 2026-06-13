//! Sealed-crate signature lockfile (spec §12.5).

pub use crisp_manifest::{
    CrispLock, LockError, ResolvedDependency, SealedSignature, read_lock, write_lock,
};
