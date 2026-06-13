use crate::types::{InferContext, Ty, TypeVar};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnifyError {
    #[error("type mismatch: expected {expected}, found {found}")]
    Mismatch { expected: String, found: String },
    #[error("occurs check failed")]
    OccursCheck,
    #[error("infinite type")]
    Infinite,
}

pub fn unify(ctx: &mut InferContext, t1: &Ty, t2: &Ty) -> Result<(), UnifyError> {
    let t1 = ctx.apply(t1);
    let t2 = ctx.apply(t2);
    match (&t1, &t2) {
        (Ty::Var(v1), Ty::Var(v2)) if v1 == v2 => Ok(()),
        (Ty::Var(v), other) | (other, Ty::Var(v)) => {
            if occurs(*v, other, &ctx.subst) {
                return Err(UnifyError::OccursCheck);
            }
            ctx.subst.insert(*v, other.clone());
            Ok(())
        }
        (Ty::Tuple(a), Ty::Tuple(b)) if a.len() == b.len() => {
            for (x, y) in a.iter().zip(b) {
                unify(ctx, x, y)?;
            }
            Ok(())
        }
        (Ty::Fn { params: a, ret: ra }, Ty::Fn { params: b, ret: rb }) if a.len() == b.len() => {
            for (x, y) in a.iter().zip(b) {
                unify(ctx, x, y)?;
            }
            unify(ctx, ra, rb)
        }
        (Ty::Option(a), Ty::Option(b)) => unify(ctx, a, b),
        (
            Ty::Ref {
                mutable: m1,
                inner: a,
            },
            Ty::Ref {
                mutable: m2,
                inner: b,
            },
        ) if m1 == m2 => unify(ctx, a, b),
        (Ty::Named { name: n1, args: a1 }, Ty::Named { name: n2, args: a2 })
            if n1 == n2 && a1.len() == a2.len() =>
        {
            for (x, y) in a1.iter().zip(a2) {
                unify(ctx, x, y)?;
            }
            Ok(())
        }
        (Ty::Array { elem: a, len: la }, Ty::Array { elem: b, len: lb }) if la == lb => {
            unify(ctx, a, b)
        }
        (Ty::Slice(a), Ty::Slice(b)) => unify(ctx, a, b),
        (a, b) if a == b => Ok(()),
        (Ty::StrSlice, Ty::Str) | (Ty::Str, Ty::StrSlice) => Ok(()),
        (Ty::Int, Ty::UInt) | (Ty::UInt, Ty::Int) => Ok(()),
        (a, b) => Err(UnifyError::Mismatch {
            expected: format!("{a:?}"),
            found: format!("{b:?}"),
        }),
    }
}

fn occurs(var: TypeVar, ty: &Ty, subst: &std::collections::HashMap<TypeVar, Ty>) -> bool {
    match ty {
        Ty::Var(v) => {
            if let Some(t) = subst.get(v) {
                occurs(var, t, subst)
            } else {
                *v == var
            }
        }
        Ty::Tuple(ts) => ts.iter().any(|t| occurs(var, t, subst)),
        Ty::Fn { params, ret } => {
            params.iter().any(|p| occurs(var, p, subst)) || occurs(var, ret, subst)
        }
        Ty::Option(inner) | Ty::Slice(inner) | Ty::Array { elem: inner, .. } => {
            occurs(var, inner, subst)
        }
        Ty::Ref { inner, .. } => occurs(var, inner, subst),
        Ty::Named { args, .. } => args.iter().any(|a| occurs(var, a, subst)),
        _ => false,
    }
}
