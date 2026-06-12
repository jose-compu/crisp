use crate::Span;

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit { value: i64, span: Span },
    // TODO: full expression grammar (spec §2, Appendix A)
}
