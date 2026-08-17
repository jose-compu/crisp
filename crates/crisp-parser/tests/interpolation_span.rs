use crisp_ast::expr::{ExprKind, StringPart};
use crisp_ast::item::Item;
use crisp_parser::Parser;

#[test]
fn interpolation_ident_span_is_inside_the_string() {
    let src = "use math.add { add }\n\npub main() = {\n    print(\"speed={s}\")\n}\n";
    let mut parser = Parser::new(src).expect("lex");
    let file = parser.parse_file().expect("parse");
    let Item::Function(f) = file
        .items
        .iter()
        .find(|i| matches!(i, Item::Function(fn_) if fn_.name.name == "main"))
        .expect("main")
    else {
        panic!("main");
    };
    let needle = src.find("{s}").expect("{s}");
    let ident_start = (needle + 1) as u32;
    let found = find_ident_span(&f.body, "s").expect("interpolation ident s");
    assert_eq!(found.start, ident_start, "src:\n{src}");
    assert_eq!(&src[found.start as usize..found.end as usize], "s");
    let line = src[..found.start as usize]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1;
    assert!(line > 1, "must not remap to line 1, got {line}");
}

fn find_ident_span(expr: &crisp_ast::expr::Expr, name: &str) -> Option<crisp_ast::Span> {
    match &expr.kind {
        ExprKind::Ident(id) if id.name == name => Some(id.span),
        ExprKind::Str(parts) => parts.0.iter().find_map(|p| match p {
            StringPart::Expr(e) => find_ident_span(e, name),
            StringPart::Lit(_) => None,
        }),
        ExprKind::Call { func, args } => find_ident_span(func, name)
            .or_else(|| args.iter().find_map(|a| find_ident_span(a, name))),
        ExprKind::Block(b) => b
            .stmts
            .iter()
            .find_map(|s| match s {
                crisp_ast::expr::Stmt::Expr(e)
                | crisp_ast::expr::Stmt::Bind { value: e, .. }
                | crisp_ast::expr::Stmt::Assign { value: e, .. } => find_ident_span(e, name),
            })
            .or_else(|| b.tail.as_ref().and_then(|t| find_ident_span(t, name))),
        _ => None,
    }
}
