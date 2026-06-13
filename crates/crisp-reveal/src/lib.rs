//! `reveal` commands — types, ownership, lifetimes, errors, traits, rust, seal (spec §16).

mod types;

pub use types::reveal_types;

#[derive(Debug, Clone, Copy)]
pub enum RevealMode {
    Types,
    Ownership,
    Lifetimes,
    Errors,
    Traits,
    Rust,
    Seal,
    Expand,
    Diff,
    Map,
}
