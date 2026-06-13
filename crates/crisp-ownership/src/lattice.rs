use crate::usage::Usage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OwnershipMode {
    Borrow = 0,
    MutBorrow = 1,
    Owned = 2,
}

impl OwnershipMode {
    pub fn join(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }

    pub fn from_explicit(ownership: Option<crisp_ast::expr::Ownership>) -> Option<Self> {
        use crisp_ast::expr::Ownership;
        match ownership {
            None => None,
            Some(Ownership::Own) => Some(OwnershipMode::Owned),
            Some(Ownership::Ref) => Some(OwnershipMode::Borrow),
            Some(Ownership::RefMut) => Some(OwnershipMode::MutBorrow),
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            OwnershipMode::Borrow => "&",
            OwnershipMode::MutBorrow => "&mut",
            OwnershipMode::Owned => "own",
        }
    }

    pub fn from_usage(usage: Usage) -> Self {
        usage.required_mode()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_ordering() {
        assert_eq!(
            OwnershipMode::join(OwnershipMode::Borrow, OwnershipMode::MutBorrow),
            OwnershipMode::MutBorrow
        );
        assert_eq!(
            OwnershipMode::join(OwnershipMode::Borrow, OwnershipMode::Owned),
            OwnershipMode::Owned
        );
    }
}
