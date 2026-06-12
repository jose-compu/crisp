use crate::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Str,
    Never,
    Option(Box<Type>),
    // TODO: full type system (spec §3)
    Named { name: String, span: Span },
}
