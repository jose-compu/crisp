use crate::{expr::Expr, ident::Ident, span::Span, ty::Type};

#[derive(Debug, Clone, PartialEq)]
pub struct Pat {
    pub kind: PatKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatKind {
    Wildcard,
    Ident(Ident),
    Literal(Box<Expr>),
    Tuple(Vec<Pat>),
    Struct {
        name: Ident,
        fields: Vec<FieldPat>,
        rest: Option<Ident>,
    },
    Enum {
        name: Ident,
        variant: Ident,
        args: Vec<Pat>,
    },
    Slice {
        prefix: Vec<Pat>,
        rest: Option<Ident>,
    },
    Type {
        inner: Box<Pat>,
        ty: Type,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPat {
    pub name: Ident,
    pub pat: Option<Pat>,
    pub span: Span,
}
