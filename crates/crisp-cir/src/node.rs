use crisp_ownership::OwnershipMode;

#[derive(Debug, Clone)]
pub struct CirNode {
    pub ownership: OwnershipMode,
    // TODO: resolved type, source span map for rustc error mapping (§17.3)
}
