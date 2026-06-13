use crate::OwnershipMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Usage {
    Read,
    Mutate,
    MoveOut,
    Copy,
}

impl Usage {
    pub fn required_mode(self) -> OwnershipMode {
        match self {
            Usage::Read | Usage::Copy => OwnershipMode::Borrow,
            Usage::Mutate => OwnershipMode::MutBorrow,
            Usage::MoveOut => OwnershipMode::Owned,
        }
    }
}

pub fn join_usage(a: Usage, b: Usage) -> Usage {
    use OwnershipMode::*;
    let mode = OwnershipMode::join(a.required_mode(), b.required_mode());
    match mode {
        Borrow => Usage::Read,
        MutBorrow => Usage::Mutate,
        Owned => Usage::MoveOut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_read_move_is_move() {
        assert_eq!(join_usage(Usage::Read, Usage::MoveOut), Usage::MoveOut);
    }

    #[test]
    fn join_read_mutate_is_mutate() {
        assert_eq!(join_usage(Usage::Read, Usage::Mutate), Usage::Mutate);
    }

    #[test]
    fn join_read_read_is_read() {
        assert_eq!(join_usage(Usage::Read, Usage::Read), Usage::Read);
    }
}
