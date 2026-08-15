//! User-facing generics parse (#71).

use crisp_ast::item::Item;
use crisp_parser::Parser;

#[test]
fn type_generics_parse() {
    let src = r#"
type Pair<A, B> = {
    left: A
    right: B
}
"#;
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("generic type should parse");
    let Item::TypeDef(td) = &file.items[0] else {
        panic!("expected TypeDef");
    };
    assert_eq!(td.generics.len(), 2);
    assert_eq!(td.generics[0].name, "A");
    assert_eq!(td.generics[1].name, "B");
}

#[test]
fn function_generics_parse() {
    let src = "id<T>(x: T) = x\n";
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("generic function should parse");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected Function");
    };
    assert_eq!(f.generics.len(), 1);
    assert_eq!(f.generics[0].name, "T");
}

#[test]
fn trait_generics_and_impl_args_parse() {
    let src = r#"
trait Wrapper<T> = {
    unwrap(self) -> T
}

impl Wrapper<int> for IntBox = {
    unwrap(self) = self.value
}
"#;
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("generic trait/impl should parse");
    let Item::TraitDef(t) = &file.items[0] else {
        panic!("expected TraitDef");
    };
    assert_eq!(t.generics.len(), 1);
    assert_eq!(t.generics[0].name, "T");
    let Item::Impl(ib) = &file.items[1] else {
        panic!("expected Impl");
    };
    assert_eq!(ib.trait_args.len(), 1);
}
