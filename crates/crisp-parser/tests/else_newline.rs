//! Unbraced `else` must not treat the next line as a postfix call (#145).

use crisp_ast::expr::{BinaryOp, ExprKind};
use crisp_ast::item::Item;
use crisp_parser::Parser;

#[test]
fn issue_145_else_div_is_not_a_call_of_den() {
    let src = r#"
frac_of(a, den, i) = {
    frac := if a > 0.0 then a / den else (0.5 - a) / den
    ((i as float) + 0.5) / den + frac
}
"#;
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("parse #145");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected function");
    };
    let ExprKind::Block(block) = &f.body.kind else {
        panic!("expected block, got {:?}", f.body.kind);
    };
    let bind = block.stmts.iter().find_map(|s| match s {
        crisp_ast::expr::Stmt::Bind { value, .. } => Some(value),
        _ => None,
    });
    let some_if = bind.expect("frac bind");
    let ExprKind::If {
        else_branch: Some(els),
        ..
    } = &some_if.kind
    else {
        panic!("expected If with else, got {:?}", some_if.kind);
    };
    match &els.kind {
        ExprKind::Binary {
            op: BinaryOp::Div, ..
        } => {}
        ExprKind::Call { .. } => panic!("else branch parsed as a call of den (#145): {els:?}"),
        other => panic!("expected Div else-branch, got {other:?}"),
    }
    assert!(
        block.tail.is_some(),
        "next line should remain a sibling tail using frac"
    );
}

#[test]
fn issue_145_same_line_call_unchanged() {
    let src = "f() = foo(1)\n";
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("parse foo(1)");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected function");
    };
    assert!(
        matches!(f.body.kind, ExprKind::Call { .. }),
        "same-line foo(1) should stay a call, got {:?}",
        f.body.kind
    );
}

#[test]
fn issue_145_else_assign_unchanged() {
    let src = r#"
step(lo, mid) = {
    if true then hi = mid else lo = mid
    lo
}
"#;
    let mut p = Parser::new(src).expect("lex");
    p.parse_file().expect("else lo = mid should parse");
}
