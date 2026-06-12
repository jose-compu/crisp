//! Name resolution and module graph (spec §12).

pub mod error;
pub mod lockfile;
pub mod module;
pub mod prelude;
pub mod resolve;
pub mod symbols;

pub use error::ResolveError;
pub use module::{ModuleGraph, find_crate_root, load_module_graph};
pub use resolve::{ResolvedBinding, ResolvedCrate, ResolvedModule, Resolver};
