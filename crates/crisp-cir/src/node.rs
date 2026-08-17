use crate::ty::CirTy;
use crisp_ast::Span;
use crisp_errors::CrispErrorEnum;
use crisp_ownership::OwnershipMode;
use crisp_regions::LifetimeSig;

#[derive(Debug, Clone)]
pub struct CirCrate {
    pub package_name: String,
    pub modules: Vec<CirModule>,
    pub crisp_error: CrispErrorEnum,
    pub shape_traits: Vec<CirShapeTrait>,
    pub source_map: crate::source_map::SourceMap,
}

#[derive(Debug, Clone)]
pub struct CirModule {
    pub path: String,
    pub items: Vec<CirItem>,
}

#[derive(Debug, Clone)]
pub enum CirItem {
    Struct(CirStruct),
    Enum(CirEnum),
    Alias {
        name: String,
        is_pub: bool,
        ty: CirTy,
        span: Span,
    },
    Function(CirFunction),
    Trait(CirTrait),
    Impl(CirImpl),
    Extern(CirExternBlock),
}

#[derive(Debug, Clone)]
pub struct CirTrait {
    pub name: String,
    pub generics: Vec<String>,
    pub methods: Vec<CirTraitMethod>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirStruct {
    pub name: String,
    pub is_pub: bool,
    pub generics: Vec<String>,
    pub fields: Vec<CirField>,
    pub with_fn: Option<CirWithFn>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirField {
    pub name: String,
    pub ty: CirTy,
    pub default: Option<CirExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirWithFn {
    pub fields: Vec<String>,
    pub defaults: Vec<Option<CirExpr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirEnum {
    pub name: String,
    pub is_pub: bool,
    pub generics: Vec<String>,
    pub variants: Vec<CirVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirVariant {
    pub name: String,
    pub fields: Vec<CirTy>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirShapeTrait {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<(String, CirTy)>,
    pub methods: Vec<CirTraitMethod>,
    pub impls: Vec<CirShapeImpl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirTraitMethod {
    pub name: String,
    pub params: Vec<(String, CirTy)>,
    pub ret: CirTy,
    /// Optional default method body (user `trait` only; shapes leave this `None`).
    pub default_body: Option<CirExpr>,
}

#[derive(Debug, Clone)]
pub struct CirShapeImpl {
    pub ty_name: String,
    pub ty_generics: Vec<String>,
    pub args: Vec<CirTy>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirImpl {
    pub trait_name: Option<String>,
    pub trait_args: Vec<CirTy>,
    pub ty_name: String,
    pub functions: Vec<CirFunction>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirFunction {
    pub name: String,
    pub is_pub: bool,
    pub is_main: bool,
    pub is_async: bool,
    pub generics: Vec<String>,
    /// `T` → `Add` / `Show` / … inferred from use (spec §15.4 / #84).
    pub op_bounds: std::collections::BTreeMap<String, Vec<String>>,
    pub params: Vec<CirParam>,
    pub ret: CirTy,
    pub fallible: bool,
    pub lifetimes: Option<LifetimeSig>,
    pub body: CirBlock,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirExternBlock {
    pub abi: String,
    pub functions: Vec<CirExternFn>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirExternFn {
    pub name: String,
    pub params: Vec<CirParam>,
    pub ret: CirTy,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirParam {
    pub name: String,
    pub ty: CirTy,
    pub mode: OwnershipMode,
    pub lifetime: Option<String>,
    /// Extra `+ Bound` names from `HasName + Show` (#77).
    pub extra_bounds: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CirBlock {
    pub stmts: Vec<CirStmt>,
    pub tail: Option<Box<CirExpr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum CirStmt {
    Let {
        name: String,
        mutable: bool,
        value: CirExpr,
        span: Span,
    },
    Expr(CirExpr),
    Assign {
        target: String,
        value: CirExpr,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum CirExpr {
    Unit {
        span: Span,
    },
    Int {
        value: i64,
        span: Span,
    },
    Float {
        value: f64,
        span: Span,
    },
    Str {
        value: String,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Ident {
        name: String,
        ty: CirTy,
        span: Span,
    },
    BinOp {
        op: CirBinOp,
        left: Box<CirExpr>,
        right: Box<CirExpr>,
        ty: CirTy,
        span: Span,
    },
    Call {
        callee: String,
        module: String,
        args: Vec<CirCallArg>,
        ty: CirTy,
        fallible: bool,
        propagate_error: bool,
        is_extern: bool,
        span: Span,
    },
    StructLit {
        name: String,
        fields: Vec<(String, CirExpr)>,
        all_fields: Vec<String>,
        use_with: bool,
        ty: CirTy,
        span: Span,
    },
    Throw {
        payload: Box<CirExpr>,
        span: Span,
    },
    Try {
        expr: Box<CirExpr>,
        span: Span,
    },
    Catch {
        expr: Box<CirExpr>,
        arms: Vec<CirCatchArm>,
        ty: CirTy,
        span: Span,
    },
    Clone {
        expr: Box<CirExpr>,
        span: Span,
    },
    Borrow {
        expr: Box<CirExpr>,
        mutable: bool,
        span: Span,
    },
    Field {
        base: Box<CirExpr>,
        field: String,
        ty: CirTy,
        span: Span,
    },
    /// Enum variant construction: `Color::Red` / `Color::Custom(r, g, b)`.
    EnumVariant {
        ty_name: String,
        variant: String,
        args: Vec<CirExpr>,
        ty: CirTy,
        span: Span,
    },
    /// Associated inherent fn: `Vec2::new(...)` (spec §5.4).
    AssocCall {
        ty_name: String,
        method: String,
        args: Vec<CirExpr>,
        ty: CirTy,
        span: Span,
    },
    /// Instance method: `recv.magnitude()` / `recv.scale(f)`.
    MethodCall {
        receiver: Box<CirExpr>,
        method: String,
        args: Vec<CirExpr>,
        ty: CirTy,
        span: Span,
    },
    Format {
        parts: Vec<CirFormatPart>,
        span: Span,
    },
    Print {
        arg: Box<CirExpr>,
        debug: bool,
        span: Span,
    },
    If {
        cond: Box<CirExpr>,
        then_branch: Box<CirExpr>,
        else_branch: Option<Box<CirExpr>>,
        ty: CirTy,
        span: Span,
    },
    Match {
        scrutinee: Box<CirExpr>,
        arms: Vec<CirMatchArm>,
        ty: CirTy,
        span: Span,
    },
    For {
        pat: CirPat,
        iter: Box<CirExpr>,
        body: Box<CirExpr>,
        span: Span,
    },
    While {
        cond: Box<CirExpr>,
        body: Box<CirExpr>,
        span: Span,
    },
    Loop {
        body: Box<CirExpr>,
        span: Span,
    },
    Break {
        value: Option<Box<CirExpr>>,
        span: Span,
    },
    Continue {
        span: Span,
    },
    Unsafe {
        body: Box<CirExpr>,
        span: Span,
    },
    Async {
        body: Box<CirExpr>,
        span: Span,
    },
    Await {
        expr: Box<CirExpr>,
        ty: CirTy,
        span: Span,
    },
    Spawn {
        expr: Box<CirExpr>,
        span: Span,
    },
    Block(CirBlock),
    /// Anonymous / local function value. Emits a Rust closure.
    Lambda {
        params: Vec<String>,
        body: Box<CirExpr>,
        ty: CirTy,
        span: Span,
    },
    /// Call a function value (local binding or lambda), not a named item.
    Apply {
        func: Box<CirExpr>,
        args: Vec<CirExpr>,
        ty: CirTy,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct CirMatchArm {
    pub pat: CirPat,
    pub guard: Option<CirExpr>,
    pub body: CirExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum CirPat {
    Wildcard {
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    Int {
        value: i64,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Str {
        value: String,
        span: Span,
    },
    Struct {
        name: String,
        fields: Vec<(String, CirPat)>,
        span: Span,
    },
    Enum {
        ty_name: String,
        variant: String,
        args: Vec<CirPat>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum CirFormatPart {
    Lit(String),
    Expr(CirExpr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CirBinOp {
    Concat,
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl CirBinOp {
    /// Higher binds tighter. Used to restore grouping that emit would otherwise drop (#99).
    pub fn rust_prec(self) -> u8 {
        match self {
            CirBinOp::Pow => 8,
            CirBinOp::Mul | CirBinOp::Div => 7,
            CirBinOp::Add | CirBinOp::Sub | CirBinOp::Concat => 6,
            CirBinOp::Eq
            | CirBinOp::Ne
            | CirBinOp::Lt
            | CirBinOp::Le
            | CirBinOp::Gt
            | CirBinOp::Ge => 3,
            CirBinOp::And => 2,
            CirBinOp::Or => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CirCallArg {
    pub expr: CirExpr,
    pub mode: OwnershipMode,
}

#[derive(Debug, Clone)]
pub struct CirCatchArm {
    pub wildcard: bool,
    pub body: CirExpr,
    pub span: Span,
}
