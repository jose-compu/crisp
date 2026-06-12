use crate::{
    expr::{Block, Expr, Param},
    ident::Ident,
    span::Span,
    ty::{ErrorType, Type},
};

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FunctionDef),
    TypeDef(TypeDef),
    TraitDef(TraitDef),
    ShapeDef(ShapeDef),
    Impl(ImplBlock),
    Use(UseDecl),
    Const(ConstDef),
    Extern(ExternBlock),
    Test(TestDef),
    TestCompileFail(TestDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub is_pub: bool,
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub params: Vec<Param>,
    pub ret_type: Option<Type>,
    pub error_type: Option<ErrorType>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub is_pub: bool,
    pub name: Ident,
    pub generics: Vec<Ident>,
    pub body: TypeBody,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeBody {
    Struct(Vec<FieldDef>),
    Enum(Vec<VariantDef>),
    Alias(Type),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: Ident,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariantDef {
    pub name: Ident,
    pub fields: Vec<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    pub name: Ident,
    pub items: Vec<TraitItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitItem {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret_type: Option<Type>,
    pub default_body: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDef {
    pub name: Ident,
    pub fields: Vec<ShapeField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShapeField {
    Data {
        name: Ident,
        ty: Type,
        span: Span,
    },
    Method {
        name: Ident,
        params: Vec<Param>,
        ret_type: Type,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    pub trait_name: Option<Ident>,
    pub ty: Type,
    pub items: Vec<FunctionDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    pub is_pub: bool,
    pub path: Vec<Ident>,
    pub imports: Option<Vec<UseImport>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseImport {
    pub name: Ident,
    pub alias: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDef {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternBlock {
    pub abi: String,
    pub functions: Vec<ExternFn>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternFn {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret_type: Option<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestDef {
    pub name: String,
    pub body: Block,
    pub span: Span,
}
