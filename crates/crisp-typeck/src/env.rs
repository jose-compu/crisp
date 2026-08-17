#![allow(dead_code)]

use crate::types::{InferContext, Scheme, Ty};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    bindings: HashMap<String, Scheme>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, scheme: Scheme) {
        self.bindings.insert(name.into(), scheme);
    }

    pub fn remove(&mut self, name: &str) -> Option<Scheme> {
        self.bindings.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&Scheme> {
        self.bindings.get(name)
    }

    pub fn extend(&mut self, other: &TypeEnv) {
        self.bindings.extend(other.bindings.clone());
    }

    pub fn free_vars(&self, ctx: &mut InferContext) -> HashSet<u32> {
        let mut set = HashSet::new();
        for scheme in self.bindings.values() {
            let ty = ctx.apply(&scheme.ty);
            collect_free(&ty, &mut set);
            for v in &scheme.vars {
                set.remove(v);
            }
        }
        set
    }
}

pub fn scheme(ty: Ty) -> Scheme {
    Scheme { vars: vec![], ty }
}

pub fn instantiate(ctx: &mut InferContext, scheme: &Scheme) -> Ty {
    let mut ty = scheme.ty.clone();
    for v in &scheme.vars {
        let fresh = ctx.fresh();
        ty = substitute_var(&ty, *v, &fresh);
    }
    ctx.apply(&ty)
}

pub fn generalize(env: &TypeEnv, ctx: &mut InferContext, ty: &Ty) -> Scheme {
    let ty = ctx.apply(ty);
    let env_free = env.free_vars(ctx);
    let mut vars = Vec::new();
    collect_free_vars(&ty, &mut vars);
    vars.retain(|v| !env_free.contains(v));
    vars.sort_unstable();
    vars.dedup();
    Scheme { vars, ty }
}

fn collect_free(ty: &Ty, set: &mut HashSet<u32>) {
    match ty {
        Ty::Var(v) => {
            set.insert(*v);
        }
        Ty::Tuple(ts) => ts.iter().for_each(|t| collect_free(t, set)),
        Ty::Fn { params, ret } => {
            params.iter().for_each(|p| collect_free(p, set));
            collect_free(ret, set);
        }
        Ty::Option(inner) | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
            collect_free(inner, set);
        }
        Ty::Ref { inner, .. } => collect_free(inner, set),
        Ty::Named { args, .. } => args.iter().for_each(|a| collect_free(a, set)),
        _ => {}
    }
}

pub(crate) fn collect_free_vars(ty: &Ty, out: &mut Vec<u32>) {
    match ty {
        Ty::Var(v) => out.push(*v),
        Ty::Tuple(ts) => ts.iter().for_each(|t| collect_free_vars(t, out)),
        Ty::Fn { params, ret } => {
            params.iter().for_each(|p| collect_free_vars(p, out));
            collect_free_vars(ret, out);
        }
        Ty::Option(inner) | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
            collect_free_vars(inner, out);
        }
        Ty::Ref { inner, .. } => collect_free_vars(inner, out),
        Ty::Named { args, .. } => args.iter().for_each(|a| collect_free_vars(a, out)),
        _ => {}
    }
}

pub(crate) fn substitute_var(ty: &Ty, var: u32, replacement: &Ty) -> Ty {
    match ty {
        Ty::Var(v) if *v == var => replacement.clone(),
        Ty::Var(_) => ty.clone(),
        Ty::Tuple(ts) => Ty::Tuple(
            ts.iter()
                .map(|t| substitute_var(t, var, replacement))
                .collect(),
        ),
        Ty::Fn { params, ret } => Ty::Fn {
            params: params
                .iter()
                .map(|p| substitute_var(p, var, replacement))
                .collect(),
            ret: Box::new(substitute_var(ret, var, replacement)),
        },
        Ty::Option(inner) => Ty::Option(Box::new(substitute_var(inner, var, replacement))),
        Ty::Slice(inner) => Ty::Slice(Box::new(substitute_var(inner, var, replacement))),
        Ty::Array { elem, len } => Ty::Array {
            elem: Box::new(substitute_var(elem, var, replacement)),
            len: *len,
        },
        Ty::Ref { mutable, inner } => Ty::Ref {
            mutable: *mutable,
            inner: Box::new(substitute_var(inner, var, replacement)),
        },
        Ty::Named { name, args } => Ty::Named {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| substitute_var(a, var, replacement))
                .collect(),
        },
        other => other.clone(),
    }
}
