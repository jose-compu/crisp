use crisp_ast::Span;
use std::collections::HashMap;

pub type TypeVar = u32;

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Var(TypeVar),
    Never,
    Unit,
    Bool,
    Int,
    UInt,
    Float,
    Char,
    Str,
    StrSlice,
    Tuple(Vec<Ty>),
    Array { elem: Box<Ty>, len: u64 },
    Slice(Box<Ty>),
    Fn { params: Vec<Ty>, ret: Box<Ty> },
    Option(Box<Ty>),
    Ref { mutable: bool, inner: Box<Ty> },
    Named { name: String, args: Vec<Ty> },
    Error,
}

impl Ty {
    pub fn is_stringish(&self) -> bool {
        matches!(self, Ty::Str | Ty::StrSlice)
    }
}

#[derive(Debug, Clone)]
pub struct Scheme {
    pub vars: Vec<TypeVar>,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub struct InferContext {
    pub next_var: TypeVar,
    pub subst: HashMap<TypeVar, Ty>,
}

impl InferContext {
    pub fn new() -> Self {
        Self {
            next_var: 0,
            subst: HashMap::new(),
        }
    }

    pub fn fresh(&mut self) -> Ty {
        let v = self.next_var;
        self.next_var += 1;
        Ty::Var(v)
    }

    pub fn apply(&mut self, ty: &Ty) -> Ty {
        match ty {
            Ty::Var(v) => {
                if let Some(t) = self.subst.get(v).cloned() {
                    let t = self.apply(&t);
                    self.subst.insert(*v, t.clone());
                    t
                } else {
                    ty.clone()
                }
            }
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.apply(t)).collect()),
            Ty::Array { elem, len } => Ty::Array {
                elem: Box::new(self.apply(elem)),
                len: *len,
            },
            Ty::Slice(inner) => Ty::Slice(Box::new(self.apply(inner))),
            Ty::Fn { params, ret } => Ty::Fn {
                params: params.iter().map(|p| self.apply(p)).collect(),
                ret: Box::new(self.apply(ret)),
            },
            Ty::Option(inner) => Ty::Option(Box::new(self.apply(inner))),
            Ty::Ref { mutable, inner } => Ty::Ref {
                mutable: *mutable,
                inner: Box::new(self.apply(inner)),
            },
            Ty::Named { name, args } => Ty::Named {
                name: name.clone(),
                args: args.iter().map(|a| self.apply(a)).collect(),
            },
            other => other.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InferredSig {
    pub module: String,
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub span: Span,
}
