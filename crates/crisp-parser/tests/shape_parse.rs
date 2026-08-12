//! Shape syntax parses; resolve accepts shape defs/bounds (#61).

use crisp_ast::item::Item;
use crisp_parser::Parser;

#[test]
fn shape_def_parses() {
    let src = r#"
shape HasPosition = {
    x: float
    y: float
}
"#;
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("shape def should parse");
    assert!(
        file.items.iter().any(|i| matches!(i, Item::ShapeDef(_))),
        "expected ShapeDef item"
    );
}

#[test]
fn shape_bound_in_param_parses() {
    let src = r#"
distance(a: T + shape HasPosition) = 0.0
"#;
    let mut p = Parser::new(src).expect("lex");
    p.parse_file().expect("shape bound should parse");
}
