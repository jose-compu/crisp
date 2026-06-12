//! Ownership pass — usage collection, lattice join, clone insertion (spec §7).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipMode {
    Borrow,
    MutBorrow,
    Owned,
}

pub struct OwnershipPass;
