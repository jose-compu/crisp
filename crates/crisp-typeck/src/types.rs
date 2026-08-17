use crisp_ast::Span;
use std::collections::{BTreeMap, HashMap};

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
    /// Inferred bounds on generics (`T` → `Add` / `Show` / …) from operators and unique trait methods (#84).
    pub op_bounds: BTreeMap<String, Vec<String>>,
}

impl InferredSig {
    /// Param types, return type, and generics as they should be emitted (#76).
    pub fn emit_view(&self) -> (Vec<(String, Ty)>, Ty, Vec<String>) {
        let Some(args) = &self.mono_args else {
            return (self.params.clone(), self.ret.clone(), self.generics.clone());
        };
        let mut subst = std::collections::BTreeMap::new();
        for ((_, sty), ity) in self.params.iter().zip(args.iter()) {
            collect_generic_subst(sty, ity, &self.generics, &mut subst);
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
        self.scheme_prefix_for(&self.generics)
    }

    pub fn scheme_prefix_for(&self, gens: &[String]) -> String {
        if gens.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                gens.iter()
                    .map(|g| self.crisp_generic_bound(g))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    fn crisp_generic_bound(&self, g: &str) -> String {
        let mut parts = vec![format!("{g}: Clone")];
        if let Some(ops) = self.op_bounds.get(g) {
            if ops.iter().any(|o| is_arith_bound(o)) {
                parts.push("Copy".into());
            }
            for op in ops {
                parts.push(op.clone());
            }
        }
        parts.join(" + ")
    }

    /// Rust binder list (`T: Clone + std::ops::Add<Output = T>`).
    pub fn rust_scheme_prefix_for(&self, gens: &[String]) -> String {
        if gens.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                gens.iter()
                    .map(|g| self.rust_generic_bound(g))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    pub fn rust_generic_bound(&self, g: &str) -> String {
        let mut parts = vec![format!("{g}: Clone")];
        if let Some(ops) = self.op_bounds.get(g) {
            if ops.iter().any(|o| is_arith_bound(o)) {
                parts.push("Copy".into());
            }
            for op in ops {
                parts.push(rust_op_bound(g, op));
            }
        }
        parts.join(" + ")
    }
}

/// Prelude arithmetic traits inferred from `+` `-` `*` `/` (spec §15.4).
pub fn is_arith_bound(name: &str) -> bool {
    matches!(name, "Add" | "Sub" | "Mul" | "Div")
}

/// Prelude operator trait → `std::ops` bound (spec §15.4).
pub fn rust_op_bound(generic: &str, op: &str) -> String {
    match op {
        "Add" => format!("std::ops::Add<Output = {generic}>"),
        "Sub" => format!("std::ops::Sub<Output = {generic}>"),
        "Mul" => format!("std::ops::Mul<Output = {generic}>"),
        "Div" => format!("std::ops::Div<Output = {generic}>"),
        other => other.to_string(),
    }
}

fn collect_generic_subst(
    scheme: &Ty,
    inst: &Ty,
    generics: &[String],
    subst: &mut std::collections::BTreeMap<String, Ty>,
) {
    match (scheme, inst) {
        (Ty::Named { name, args }, inst)
            if args.is_empty() && generics.iter().any(|g| g == name) =>
        {
            subst.entry(name.clone()).or_insert_with(|| inst.clone());
        }
        (Ty::Fn { params: a, ret: ra }, Ty::Fn { params: b, ret: rb }) if a.len() == b.len() => {
            for (x, y) in a.iter().zip(b.iter()) {
                collect_generic_subst(x, y, generics, subst);
            }
            collect_generic_subst(ra, rb, generics, subst);
        }
        (Ty::Named { args: a, .. }, Ty::Named { args: b, .. }) if a.len() == b.len() => {
            for (x, y) in a.iter().zip(b.iter()) {
                collect_generic_subst(x, y, generics, subst);
            }
        }
        (Ty::Option(a), Ty::Option(b))
        | (Ty::Slice(a), Ty::Slice(b))
        | (Ty::Ref { inner: a, .. }, Ty::Ref { inner: b, .. }) => {
            collect_generic_subst(a, b, generics, subst);
        }
        (Ty::Tuple(a), Ty::Tuple(b)) if a.len() == b.len() => {
            for (x, y) in a.iter().zip(b.iter()) {
                collect_generic_subst(x, y, generics, subst);
            }
        }
        _ => {}
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
