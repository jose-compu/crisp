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
    /// When set, this signature is an inherent `impl Type` method (§5.4).
    pub impl_ty: Option<String>,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub span: Span,
    /// Explicit or inferred type parameters (`id(x) = x` → `["T"]`).
    pub generics: Vec<String>,
    pub is_pub: bool,
    /// True when generics were named from leftover free vars (`id(x) = x`), not a pin.
    pub inferred_from_use: bool,
    /// Distinct concrete call-site instantiations (`int`, `str`) for reveal.
    pub instantiations: Vec<String>,
    /// Single concrete instantiation for crate-internal emit (#76). Scheme stays generic.
    pub mono_args: Option<Vec<Ty>>,
}

impl InferredSig {
    /// Param types, return type, and generics as they should be emitted (#76).
    pub fn emit_view(&self) -> (Vec<(String, Ty)>, Ty, Vec<String>) {
        let Some(args) = &self.mono_args else {
            return (self.params.clone(), self.ret.clone(), self.generics.clone());
        };
        let mut subst = std::collections::BTreeMap::new();
        for ((_, sty), ity) in self.params.iter().zip(args.iter()) {
            if let Ty::Named { name, args } = sty
                && args.is_empty()
                && self.generics.iter().any(|g| g == name)
            {
                subst.insert(name.clone(), ity.clone());
            }
        }
        let params = self
            .params
            .iter()
            .zip(args.iter())
            .map(|((n, _), t)| (n.clone(), t.clone()))
            .collect();
        let ret = subst_named(&self.ret, &subst);
        (params, ret, Vec::new())
    }

    /// Emit-style binder list, including the hidden `T: Clone` bound (#78).
    pub fn scheme_prefix(&self) -> String {
        if self.generics.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                self.generics
                    .iter()
                    .map(|g| format!("{g}: Clone"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

fn subst_named(ty: &Ty, subst: &std::collections::BTreeMap<String, Ty>) -> Ty {
    match ty {
        Ty::Named { name, args } if args.is_empty() => {
            subst.get(name).cloned().unwrap_or_else(|| ty.clone())
        }
        Ty::Named { name, args } => Ty::Named {
            name: name.clone(),
            args: args.iter().map(|a| subst_named(a, subst)).collect(),
        },
        Ty::Fn { params, ret } => Ty::Fn {
            params: params.iter().map(|p| subst_named(p, subst)).collect(),
            ret: Box::new(subst_named(ret, subst)),
        },
        Ty::Option(inner) => Ty::Option(Box::new(subst_named(inner, subst))),
        Ty::Slice(inner) => Ty::Slice(Box::new(subst_named(inner, subst))),
        Ty::Array { elem, len } => Ty::Array {
            elem: Box::new(subst_named(elem, subst)),
            len: *len,
        },
        Ty::Ref { mutable, inner } => Ty::Ref {
            mutable: *mutable,
            inner: Box::new(subst_named(inner, subst)),
        },
        Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| subst_named(t, subst)).collect()),
        other => other.clone(),
    }
}
