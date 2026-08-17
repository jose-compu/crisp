//! Parser coverage for loop constructs (spec §6.3) and v1.7.1 #96.

use crisp_ast::expr::{ExprKind, Stmt};
use crisp_ast::item::Item;
use crisp_parser::Parser;

fn parse_fn_body(src: &str) -> crisp_ast::expr::Expr {
    let file = format!("f() = {src}");
    let mut p = Parser::new(&file).unwrap_or_else(|e| panic!("parser: {e}"));
    let ast = p.parse_file().unwrap_or_else(|e| panic!("parse: {e}"));
    let Item::Function(f) = &ast.items[0] else {
        panic!("expected function");
    };
    f.body.clone()
}

#[test]
fn parse_while_no_struct_lit_in_cond() {
    let e = parse_fn_body("{ while i < n { i = i + 1 } }");
    let ExprKind::Block(b) = e.kind else {
        panic!("{:?}", e.kind);
    };
    let w = match b.stmts.first() {
        Some(Stmt::Expr(w)) => w,
        _ => b.tail.as_ref().expect("while"),
    };
    assert!(matches!(w.kind, ExprKind::While { .. }));
}

#[test]
fn parse_for() {
    let e = parse_fn_body("{ for x in xs { total = total + x } }");
    let ExprKind::Block(b) = e.kind else {
        panic!("{:?}", e.kind);
    };
    let f = match b.stmts.first() {
        Some(Stmt::Expr(f)) => f,
        _ => b.tail.as_ref().expect("for"),
    };
    assert!(matches!(f.kind, ExprKind::For { .. }));
}

#[test]
fn parse_loop_break_value() {
    let e = parse_fn_body("{ loop { if done then break 42 } }");
    let ExprKind::Block(b) = e.kind else {
        panic!("{:?}", e.kind);
    };
    let l = match b.stmts.first() {
        Some(Stmt::Expr(l)) => l,
        _ => b.tail.as_ref().expect("loop"),
    };
    let ExprKind::Loop(body) = &l.kind else {
        panic!("{:?}", l.kind);
    };
    let ExprKind::Block(lb) = &body.kind else {
        panic!("{:?}", body.kind);
    };
    let iff = match lb.stmts.first() {
        Some(Stmt::Expr(e)) => e,
        _ => lb.tail.as_ref().expect("if"),
    };
    let ExprKind::If {
        then_branch,
        else_branch: None,
        ..
    } = &iff.kind
    else {
        panic!("{:?}", iff.kind);
    };
    assert!(matches!(then_branch.kind, ExprKind::Break(Some(_))));
}

#[test]
fn parse_continue() {
    let e = parse_fn_body("{ while true { continue } }");
    let ExprKind::Block(b) = e.kind else {
        panic!("{:?}", e.kind);
    };
    let w = match b.stmts.first() {
        Some(Stmt::Expr(w)) => w,
        _ => b.tail.as_ref().expect("while"),
    };
    let ExprKind::While { body, .. } = &w.kind else {
        panic!("{:?}", w.kind);
    };
    let ExprKind::Block(wb) = &body.kind else {
        panic!("{:?}", body.kind);
    };
    let c_kind = if let Some(t) = &wb.tail {
        &t.kind
    } else if let Some(Stmt::Expr(e)) = wb.stmts.first() {
        &e.kind
    } else {
        panic!("expected continue in while body: {:?}", wb);
    };
    assert!(matches!(c_kind, ExprKind::Continue));
}

#[test]
fn issue_96_while_then_paren_tail_is_not_a_call() {
    let e = parse_fn_body("{ while i < n { i = i + 1 } (lo + hi) / 2.0 }");
    let ExprKind::Block(b) = e.kind else {
        panic!("{:?}", e.kind);
    };
    let w = b.stmts.iter().find_map(|s| match s {
        Stmt::Expr(e) if matches!(e.kind, ExprKind::While { .. }) => Some(e),
        _ => None,
    });
    assert!(
        w.is_some(),
        "while must be a statement, not a callee (#96): {:?}",
        b
    );
    let tail = b.tail.as_ref().expect("parenthesized tail");
    assert!(
        matches!(tail.kind, ExprKind::Binary { .. }),
        "tail should be (lo + hi) / 2.0, got {:?}",
        tail.kind
    );
}

#[test]
fn issue_96_if_then_assign_parses_as_assign_expr() {
    let e = parse_fn_body("{ if ignites(mid, 0.8) then hi = mid else lo = mid }");
    let ExprKind::Block(b) = e.kind else {
        panic!("{:?}", e.kind);
    };
    let iff = match b.stmts.first() {
        Some(Stmt::Expr(e)) => e,
        _ => b.tail.as_ref().expect("if"),
    };
    let ExprKind::If {
        then_branch,
        else_branch: Some(else_branch),
        ..
    } = &iff.kind
    else {
        panic!("{:?}", iff.kind);
    };
    assert!(
        matches!(then_branch.kind, ExprKind::Assign { .. }),
        "then: {:?}",
        then_branch.kind
    );
    assert!(
        matches!(else_branch.kind, ExprKind::Assign { .. }),
        "else: {:?}",
        else_branch.kind
    );
}

fn if_ident_name(e: &crisp_ast::expr::Expr) -> &str {
    match &e.kind {
        ExprKind::Ident(id) => id.name.as_str(),
        other => panic!("expected ident condition, got {other:?}"),
    }
}

/// #117: `else if` must be an If whose *else* is another If, not an If used as the condition.
#[test]
fn parse_else_if_then_form() {
    let e = parse_fn_body("if a then 1 else if b then 2 else 3");
    let ExprKind::If {
        cond,
        then_branch,
        else_branch,
    } = &e.kind
    else {
        panic!("outer: {:?}", e.kind);
    };
    assert_eq!(if_ident_name(cond), "a");
    assert!(matches!(then_branch.kind, ExprKind::Int(1)));
    let inner = else_branch.as_ref().expect("else if");
    let ExprKind::If {
        cond: cond2,
        then_branch: then2,
        else_branch: else2,
    } = &inner.kind
    else {
        panic!("else branch should be If, got {:?}", inner.kind);
    };
    assert_eq!(if_ident_name(cond2), "b", "inner cond must not be an If");
    assert!(matches!(then2.kind, ExprKind::Int(2)));
    let else3 = else2.as_ref().expect("final else");
    assert!(matches!(else3.kind, ExprKind::Int(3)));
}

#[test]
fn parse_else_if_brace_form() {
    let e = parse_fn_body("{ if a { 1 } else if b { 2 } else { 3 } }");
    let ExprKind::Block(b) = &e.kind else {
        panic!("{:?}", e.kind);
    };
    let iff = match b.stmts.first() {
        Some(Stmt::Expr(e)) => e,
        _ => b.tail.as_ref().expect("if"),
    };
    let ExprKind::If {
        cond, else_branch, ..
    } = &iff.kind
    else {
        panic!("outer: {:?}", iff.kind);
    };
    assert_eq!(if_ident_name(cond), "a");
    let inner = else_branch.as_ref().expect("else if");
    let ExprKind::If { cond: cond2, .. } = &inner.kind else {
        panic!("else branch should be If, got {:?}", inner.kind);
    };
    assert_eq!(if_ident_name(cond2), "b");
}
