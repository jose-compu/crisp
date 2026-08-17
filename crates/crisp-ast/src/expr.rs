use crate::{ident::Ident, pat::Pat, span::Span, ty::Type};

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(StringParts),
    Unit,
    Ident(Ident),
    Block(Block),
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    For {
        pat: Pat,
        iter: Box<Expr>,
        body: Box<Expr>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    Loop(Box<Expr>),
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
    },
    Field {
        base: Box<Expr>,
        field: Ident,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    /// Postfix `expr as float` / `expr as int` (#112).
    Cast {
        expr: Box<Expr>,
        ty: crate::ty::Type,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Assign {
        target: Ident,
        value: Box<Expr>,
    },
    Bind {
        pat: Pat,
        mutable: bool,
        value: Box<Expr>,
    },
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Return(Option<Box<Expr>>),
    /// Bare `break` or `break <expr>` (value-producing `loop`, spec §6.3).
    Break(Option<Box<Expr>>),
    Continue,
    Throw(Box<Expr>),
    Catch {
        body: Box<Expr>,
        arms: Vec<CatchArm>,
    },
    Async(Box<Expr>),
    Await(Box<Expr>),
    Spawn(Box<Expr>),
    Unsafe(Box<Expr>),
    Try(Box<Expr>),
    StructLit {
        name: Ident,
        fields: Vec<FieldInit>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Lit(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringParts(pub Vec<StringPart>);

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Bind {
        pat: Pat,
        mutable: bool,
        value: Expr,
    },
    Assign {
        target: Ident,
        value: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchArm {
    pub pat: Pat,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub lifetime: Option<Ident>,
    pub ownership: Option<Ownership>,
    pub name: Ident,
    pub ty: Option<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Own,
    Ref,
    RefMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Concat,
}

impl BinaryOp {
    /// Higher binds tighter. Mirrors Rust so probe/harness emit can keep source grouping (#99).
    pub fn rust_prec(self) -> u8 {
        match self {
            BinaryOp::Pow => 8,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 7,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Concat => 6,
            BinaryOp::Shl | BinaryOp::Shr => 5,
            BinaryOp::BitAnd => 4,
            BinaryOp::BitXor => 3,
            BinaryOp::BitOr => 2,
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => 2,
            BinaryOp::And => 1,
            BinaryOp::Or => 0,
        }
    }
}
