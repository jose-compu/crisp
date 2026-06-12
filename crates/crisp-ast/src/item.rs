use crate::{expr::Expr, Span};

#[derive(Debug, Clone)]
pub enum Item {
    FnDef {
        name: String,
        body: Expr,
        span: Span,
        is_pub: bool,
    },
    // TODO: type, trait, shape, impl, test, ...
}
