//! Placeholder holes (`_`) for implicit closures (#87).
//!
//! `_` in expression position is not a name. When a function value is expected
//! (or a hole-expression is bound), holes lift left-to-right to `| _h0, _h1, … |`.
//! Nested explicit `|…|` lambdas are left alone.

use crate::expr::{
    Block, CatchArm, Expr, ExprKind, FieldInit, MatchArm, Param, Stmt, StringPart, StringParts,
};
use crate::ident::Ident;
use crate::span::Span;

pub fn is_hole_ident(name: &str) -> bool {
    name == "_"
}

pub fn count_holes(expr: &Expr) -> usize {
    let mut n = 0;
    count_in(expr, &mut n);
    n
}

pub fn lift_holes(expr: &Expr) -> Option<Expr> {
    let n = count_holes(expr);
    if n == 0 {
        return None;
    }
    let mut i = 0usize;
    let body = replace_holes(expr, &mut i);
    let params: Vec<Param> = (0..n)
        .map(|k| param_at(format!("_h{k}"), expr.span))
        .collect();
    Some(Expr {
        kind: ExprKind::Lambda {
            params,
            body: Box::new(body),
        },
        span: expr.span,
    })
}

fn param_at(name: String, span: Span) -> Param {
    Param {
        lifetime: None,
        ownership: None,
        name: Ident::new(name, span),
        ty: None,
        span,
    }
}

fn count_in(expr: &Expr, n: &mut usize) {
    match &expr.kind {
        ExprKind::Ident(id) if is_hole_ident(&id.name) => *n += 1,
        ExprKind::Lambda { .. } => {}
        ExprKind::Block(b) => count_block(b, n),
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            count_in(cond, n);
            count_in(then_branch, n);
            if let Some(e) = else_branch {
                count_in(e, n);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            count_in(scrutinee, n);
            for a in arms {
                if let Some(g) = &a.guard {
                    count_in(g, n);
                }
                count_in(&a.body, n);
            }
        }
        ExprKind::For { iter, body, .. } => {
            count_in(iter, n);
            count_in(body, n);
        }
        ExprKind::While { cond, body } => {
            count_in(cond, n);
            count_in(body, n);
        }
        ExprKind::Loop(inner)
        | ExprKind::Return(Some(inner))
        | ExprKind::Break(Some(inner))
        | ExprKind::Throw(inner)
        | ExprKind::Async(inner)
        | ExprKind::Await(inner)
        | ExprKind::Spawn(inner)
        | ExprKind::Unsafe(inner)
        | ExprKind::Try(inner)
        | ExprKind::Unary { expr: inner, .. }
        | ExprKind::Cast { expr: inner, .. } => count_in(inner, n),
        ExprKind::Call { func, .. } => count_in(func, n),
        ExprKind::MethodCall { receiver, .. } => count_in(receiver, n),
        ExprKind::Field { base, .. } => count_in(base, n),
        ExprKind::Index { base, index } => {
            count_in(base, n);
            count_in(index, n);
        }
        ExprKind::Array(elems) => {
            for e in elems {
                count_in(e, n);
            }
        }
        ExprKind::Binary { left, right, .. } | ExprKind::Pipe { left, right } => {
            count_in(left, n);
            count_in(right, n);
        }
        ExprKind::Assign { value, .. } | ExprKind::Bind { value, .. } => count_in(value, n),
        ExprKind::Catch { body, arms } => {
            count_in(body, n);
            for a in arms {
                count_in(&a.body, n);
            }
        }
        ExprKind::StructLit { fields, .. } => {
            for f in fields {
                count_in(&f.value, n);
            }
        }
        ExprKind::Str(parts) => {
            for p in &parts.0 {
                if let StringPart::Expr(e) = p {
                    count_in(e, n);
                }
            }
        }
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Unit
        | ExprKind::Ident(_)
        | ExprKind::Return(None)
        | ExprKind::Break(None)
        | ExprKind::Continue => {}
    }
}

fn count_block(block: &Block, n: &mut usize) {
    for s in &block.stmts {
        match s {
            Stmt::Expr(e) | Stmt::Bind { value: e, .. } | Stmt::Assign { value: e, .. } => {
                count_in(e, n);
            }
        }
    }
    if let Some(t) = &block.tail {
        count_in(t, n);
    }
}

fn replace_holes(expr: &Expr, i: &mut usize) -> Expr {
    let kind = match &expr.kind {
        ExprKind::Ident(id) if is_hole_ident(&id.name) => {
            let name = format!("_h{i}");
            *i += 1;
            ExprKind::Ident(Ident::new(name, id.span))
        }
        ExprKind::Lambda { .. } => return expr.clone(),
        ExprKind::Block(b) => ExprKind::Block(replace_block(b, i)),
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => ExprKind::If {
            cond: Box::new(replace_holes(cond, i)),
            then_branch: Box::new(replace_holes(then_branch, i)),
            else_branch: else_branch.as_ref().map(|e| Box::new(replace_holes(e, i))),
        },
        ExprKind::Match { scrutinee, arms } => ExprKind::Match {
            scrutinee: Box::new(replace_holes(scrutinee, i)),
            arms: arms
                .iter()
                .map(|a| MatchArm {
                    pat: a.pat.clone(),
                    guard: a.guard.as_ref().map(|g| replace_holes(g, i)),
                    body: replace_holes(&a.body, i),
                    span: a.span,
                })
                .collect(),
        },
        ExprKind::For { pat, iter, body } => ExprKind::For {
            pat: pat.clone(),
            iter: Box::new(replace_holes(iter, i)),
            body: Box::new(replace_holes(body, i)),
        },
        ExprKind::While { cond, body } => ExprKind::While {
            cond: Box::new(replace_holes(cond, i)),
            body: Box::new(replace_holes(body, i)),
        },
        ExprKind::Loop(inner) => ExprKind::Loop(Box::new(replace_holes(inner, i))),
        ExprKind::Call { func, args } => ExprKind::Call {
            func: Box::new(replace_holes(func, i)),
            args: args.iter().map(|a| replace_holes(a, i)).collect(),
        },
        ExprKind::MethodCall {
            receiver,
            method,
            args,
        } => ExprKind::MethodCall {
            receiver: Box::new(replace_holes(receiver, i)),
            method: method.clone(),
            args: args.iter().map(|a| replace_holes(a, i)).collect(),
        },
        ExprKind::Field { base, field } => ExprKind::Field {
            base: Box::new(replace_holes(base, i)),
            field: field.clone(),
        },
        ExprKind::Index { base, index } => ExprKind::Index {
            base: Box::new(replace_holes(base, i)),
            index: Box::new(replace_holes(index, i)),
        },
        ExprKind::Array(elems) => {
            ExprKind::Array(elems.iter().map(|e| replace_holes(e, i)).collect())
        }
        ExprKind::Unary { op, expr: inner } => ExprKind::Unary {
            op: *op,
            expr: Box::new(replace_holes(inner, i)),
        },
        ExprKind::Cast { expr: inner, ty } => ExprKind::Cast {
            expr: Box::new(replace_holes(inner, i)),
            ty: ty.clone(),
        },
        ExprKind::Binary { op, left, right } => ExprKind::Binary {
            op: *op,
            left: Box::new(replace_holes(left, i)),
            right: Box::new(replace_holes(right, i)),
        },
        ExprKind::Assign { target, value } => ExprKind::Assign {
            target: target.clone(),
            value: Box::new(replace_holes(value, i)),
        },
        ExprKind::Bind {
            pat,
            mutable,
            value,
        } => ExprKind::Bind {
            pat: pat.clone(),
            mutable: *mutable,
            value: Box::new(replace_holes(value, i)),
        },
        ExprKind::Pipe { left, right } => ExprKind::Pipe {
            left: Box::new(replace_holes(left, i)),
            right: Box::new(replace_holes(right, i)),
        },
        ExprKind::Return(Some(inner)) => ExprKind::Return(Some(Box::new(replace_holes(inner, i)))),
        ExprKind::Break(Some(inner)) => ExprKind::Break(Some(Box::new(replace_holes(inner, i)))),
        ExprKind::Throw(inner) => ExprKind::Throw(Box::new(replace_holes(inner, i))),
        ExprKind::Catch { body, arms } => ExprKind::Catch {
            body: Box::new(replace_holes(body, i)),
            arms: arms
                .iter()
                .map(|a| CatchArm {
                    pat: a.pat.clone(),
                    body: replace_holes(&a.body, i),
                    span: a.span,
                })
                .collect(),
        },
        ExprKind::Async(inner) => ExprKind::Async(Box::new(replace_holes(inner, i))),
        ExprKind::Await(inner) => ExprKind::Await(Box::new(replace_holes(inner, i))),
        ExprKind::Spawn(inner) => ExprKind::Spawn(Box::new(replace_holes(inner, i))),
        ExprKind::Unsafe(inner) => ExprKind::Unsafe(Box::new(replace_holes(inner, i))),
        ExprKind::Try(inner) => ExprKind::Try(Box::new(replace_holes(inner, i))),
        ExprKind::StructLit { name, fields } => ExprKind::StructLit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|f| FieldInit {
                    name: f.name.clone(),
                    value: replace_holes(&f.value, i),
                    span: f.span,
                })
                .collect(),
        },
        ExprKind::Str(parts) => ExprKind::Str(StringParts(
            parts
                .0
                .iter()
                .map(|p| match p {
                    StringPart::Lit(s) => StringPart::Lit(s.clone()),
                    StringPart::Expr(e) => StringPart::Expr(Box::new(replace_holes(e, i))),
                })
                .collect(),
        )),
        _ => return expr.clone(),
    };
    Expr {
        kind,
        span: expr.span,
    }
}

fn replace_block(block: &Block, i: &mut usize) -> Block {
    Block {
        stmts: block
            .stmts
            .iter()
            .map(|s| match s {
                Stmt::Expr(e) => Stmt::Expr(replace_holes(e, i)),
                Stmt::Bind {
                    pat,
                    mutable,
                    value,
                } => Stmt::Bind {
                    pat: pat.clone(),
                    mutable: *mutable,
                    value: replace_holes(value, i),
                },
                Stmt::Assign { target, value } => Stmt::Assign {
                    target: target.clone(),
                    value: replace_holes(value, i),
                },
            })
            .collect(),
        tail: block.tail.as_ref().map(|t| Box::new(replace_holes(t, i))),
        span: block.span,
    }
}
