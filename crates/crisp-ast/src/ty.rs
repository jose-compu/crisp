use crate::{ident::Ident, span::Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Named(Ident),
    Never,
    Unit,
    Tuple(Vec<Type>),
    Array {
        elem: Box<Type>,
        len: u64,
    },
    Slice(Box<Type>),
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Option(Box<Type>),
    Ref {
        mutable: bool,
        inner: Box<Type>,
    },
    Generic {
        base: Box<Type>,
        args: Vec<Type>,
    },
    Constrained {
        inner: Box<Type>,
        bounds: Vec<TypeBound>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeBound {
    Shape(Ident),
    Trait(Ident),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorType {
    pub variants: Vec<Type>,
    pub span: Span,
}
