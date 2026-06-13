//! AST walk utilities for position-sensitive queries.

use crisp_ast::expr::{Expr, ExprKind, Stmt};
use crisp_ast::ident::Ident;
use crisp_ast::item::{FunctionDef, Item, SourceFile};
use crisp_ast::Span;

#[derive(Debug, Clone)]
pub enum Located<'a> {
    Ident(&'a Ident),
    Call { callee: &'a Ident, args: &'a [Expr] },
    Function(&'a FunctionDef),
    Expr(&'a Expr),
}

pub fn locate_at_offset(file: &SourceFile, offset: u32) -> Option<Located<'_>> {
    let mut best: Option<(u32, Located<'_>)> = None;
    for item in &file.items {
        walk_item(item, offset, &mut best);
    }
    best.map(|(_, l)| l)
}

pub fn all_calls(file: &SourceFile) -> Vec<(Span, Ident, Vec<Expr>)> {
    let mut out = Vec::new();
    for item in &file.items {
        collect_calls_item(item, &mut out);
    }
    out
}

pub fn all_bindings(file: &SourceFile) -> Vec<(Span, String, Expr)> {
    let mut out = Vec::new();
    for item in &file.items {
        collect_bindings_item(item, &mut out);
    }
    out
}

pub fn all_functions(file: &SourceFile) -> Vec<&FunctionDef> {
    let mut out = Vec::new();
    for item in &file.items {
        if let Item::Function(f) = item {
            out.push(f);
        }
    }
    out
}

fn consider<'a>(
    best: &mut Option<(u32, Located<'a>)>,
    offset: u32,
    span: Span,
    loc: Located<'a>,
) {
    if !span.contains(offset) {
        return;
    }
    let len = span.len();
    if best.as_ref().is_none_or(|(l, _)| len < *l) {
        *best = Some((len, loc));
    }
}

fn walk_item<'a>(item: &'a Item, offset: u32, best: &mut Option<(u32, Located<'a>)>) {
    match item {
        Item::Function(f) => {
            if f.name.span.contains(offset) {
                consider(best, offset, f.name.span, Located::Function(f));
            }
            if f.span.contains(offset) {
                consider(best, offset, f.span, Located::Function(f));
            }
            walk_expr(&f.body, offset, best);
        }
        Item::Test(t) => {
            for stmt in &t.body.stmts {
                walk_stmt(stmt, offset, best);
            }
            if let Some(tail) = &t.body.tail {
                walk_expr(tail, offset, best);
            }
        }
        _ => {}
    }
}

fn walk_stmt<'a>(stmt: &'a Stmt, offset: u32, best: &mut Option<(u32, Located<'a>)>) {
    match stmt {
        Stmt::Expr(e) => walk_expr(e, offset, best),
        Stmt::Bind { pat, value, .. } => {
            if let crisp_ast::pat::PatKind::Ident(id) = &pat.kind {
                if id.span.contains(offset) {
                    consider(best, offset, id.span, Located::Ident(id));
                }
            }
            walk_expr(value, offset, best);
        }
        Stmt::Assign { target, value, .. } => {
            if target.span.contains(offset) {
                consider(best, offset, target.span, Located::Ident(target));
            }
            walk_expr(value, offset, best);
        }
    }
}

fn walk_expr<'a>(expr: &'a Expr, offset: u32, best: &mut Option<(u32, Located<'a>)>) {
    if expr.span.contains(offset) {
        consider(best, offset, expr.span, Located::Expr(expr));
    }
    match &expr.kind {
        ExprKind::Ident(id) => {
            if id.span.contains(offset) {
                consider(best, offset, id.span, Located::Ident(id));
            }
        }
        ExprKind::Call { func, args } => {
            if let ExprKind::Ident(id) = &func.kind {
                if id.span.contains(offset) {
                    consider(
                        best,
                        offset,
                        id.span,
                        Located::Call {
                            callee: id,
                            args,
                        },
                    );
                }
            }
            walk_expr(func, offset, best);
            for arg in args {
                walk_expr(arg, offset, best);
            }
        }
        ExprKind::Field { base, field } => {
            walk_expr(base, offset, best);
            if field.span.contains(offset) {
                consider(best, offset, field.span, Located::Ident(field));
            }
        }
        ExprKind::Block(b) => {
            for stmt in &b.stmts {
                walk_stmt(stmt, offset, best);
            }
            if let Some(tail) = &b.tail {
                walk_expr(tail, offset, best);
            }
        }
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            walk_expr(cond, offset, best);
            walk_expr(then_branch, offset, best);
            if let Some(e) = else_branch {
                walk_expr(e, offset, best);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            walk_expr(scrutinee, offset, best);
            for arm in arms {
                walk_expr(&arm.body, offset, best);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            walk_expr(left, offset, best);
            walk_expr(right, offset, best);
        }
        ExprKind::Unary { expr: inner, .. } => walk_expr(inner, offset, best),
        ExprKind::StructLit { name, fields } => {
            if name.span.contains(offset) {
                consider(best, offset, name.span, Located::Ident(name));
            }
            for f in fields {
                walk_expr(&f.value, offset, best);
            }
        }
        ExprKind::Catch { body, arms } => {
            walk_expr(body, offset, best);
            for arm in arms {
                walk_expr(&arm.body, offset, best);
            }
        }
        ExprKind::Async(inner)
        | ExprKind::Await(inner)
        | ExprKind::Unsafe(inner)
        | ExprKind::Spawn(inner)
        | ExprKind::Try(inner)
        | ExprKind::Throw(inner) => walk_expr(inner, offset, best),
        ExprKind::Bind { pat, value, .. } => {
            if let crisp_ast::pat::PatKind::Ident(id) = &pat.kind {
                if id.span.contains(offset) {
                    consider(best, offset, id.span, Located::Ident(id));
                }
            }
            walk_expr(value, offset, best);
        }
        _ => {}
    }
}

fn collect_calls_item(item: &Item, out: &mut Vec<(Span, Ident, Vec<Expr>)>) {
    if let Item::Function(f) = item {
        collect_calls_expr(&f.body, out);
    }
}

fn collect_calls_expr(expr: &Expr, out: &mut Vec<(Span, Ident, Vec<Expr>)>) {
    if let ExprKind::Call { func, args } = &expr.kind {
        if let ExprKind::Ident(id) = &func.kind {
            out.push((expr.span, id.clone(), args.clone()));
        }
    }
    walk_expr_collect(expr, out);
}

fn walk_expr_collect(expr: &Expr, out: &mut Vec<(Span, Ident, Vec<Expr>)>) {
    match &expr.kind {
        ExprKind::Block(b) => {
            for stmt in &b.stmts {
                if let Stmt::Expr(e) = stmt {
                    collect_calls_expr(e, out);
                } else if let Stmt::Bind { value, .. } = stmt {
                    collect_calls_expr(value, out);
                }
            }
            if let Some(t) = &b.tail {
                collect_calls_expr(t, out);
            }
        }
        ExprKind::Call { func, args, .. } => {
            walk_expr_collect(func, out);
            for a in args {
                walk_expr_collect(a, out);
            }
        }
        ExprKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            walk_expr_collect(then_branch, out);
            if let Some(e) = else_branch {
                walk_expr_collect(e, out);
            }
        }
        ExprKind::Match { arms, .. } => {
            for arm in arms {
                collect_calls_expr(&arm.body, out);
            }
        }
        ExprKind::Catch { body, arms } => {
            collect_calls_expr(body, out);
            for arm in arms {
                collect_calls_expr(&arm.body, out);
            }
        }
        ExprKind::Async(e)
        | ExprKind::Await(e)
        | ExprKind::Unsafe(e)
        | ExprKind::Spawn(e)
        | ExprKind::Try(e)
        | ExprKind::Throw(e) => walk_expr_collect(e, out),
        ExprKind::Binary { left, right, .. } => {
            walk_expr_collect(left, out);
            walk_expr_collect(right, out);
        }
        ExprKind::Field { base, .. } => walk_expr_collect(base, out),
        _ => {}
    }
}

fn collect_bindings_item(item: &Item, out: &mut Vec<(Span, String, Expr)>) {
    if let Item::Function(f) = item {
        collect_bindings_expr(&f.body, out);
    }
}

fn collect_bindings_expr(expr: &Expr, out: &mut Vec<(Span, String, Expr)>) {
    match &expr.kind {
        ExprKind::Block(b) => {
            for stmt in &b.stmts {
                if let Stmt::Bind { pat, value, .. } = stmt {
                    if let crisp_ast::pat::PatKind::Ident(id) = &pat.kind {
                        out.push((id.span, id.name.clone(), value.clone()));
                    }
                }
            }
            if let Some(t) = &b.tail {
                collect_bindings_expr(t, out);
            }
        }
        _ => {}
    }
}
