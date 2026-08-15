//! User-facing generics parse (#70 / #71).

use crisp_ast::item::{Item, ShapeField};
use crisp_ast::ty::TypeKind;
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
fn multi_function_generics_and_type_app_parse() {
    let src = "first<A, B>(p: Pair<A, B>) = p.left\n";
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("generic fn + type app should parse");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected Function");
    };
    assert_eq!(f.generics.len(), 2);
    let ty = f.params[0].ty.as_ref().expect("param type");
    assert!(
        matches!(ty.kind, TypeKind::Generic { .. }),
        "expected Pair<A, B> application, got {:?}",
        ty.kind
    );
}

#[test]
fn shape_generics_parse() {
    let src = r#"
shape Boxy<T> = {
    value: T
}
"#;
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("parametric shape should parse");
    let Item::ShapeDef(s) = &file.items[0] else {
        panic!("expected ShapeDef");
    };
    assert_eq!(s.generics.len(), 1);
    assert_eq!(s.generics[0].name, "T");
    assert!(matches!(s.fields[0], ShapeField::Data { .. }));
}

#[test]
fn shape_application_in_param_parses() {
    let src = "unwrap_int(b: Boxy<int>) = b.value\n";
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("Boxy<int> param should parse");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected Function");
    };
    let ty = f.params[0].ty.as_ref().expect("param type");
    match &ty.kind {
        TypeKind::Generic { args, .. } => assert_eq!(args.len(), 1),
        other => panic!("expected Generic application, got {other:?}"),
    }
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

#[test]
fn function_generics_with_return_type_parse() {
    let src = "id<T>(x: T) -> T = x\n";
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("generic fn with return should parse");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected Function");
    };
    assert_eq!(f.generics[0].name, "T");
    let ret = f.ret_type.as_ref().expect("return type");
    assert!(matches!(ret.kind, TypeKind::Named(_)));
}

#[test]
fn mixed_type_application_parses() {
    let src = "wrap<T>(x: Pair<T, int>) = x\n";
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("mixed type app should parse");
    let Item::Function(f) = &file.items[0] else {
        panic!("expected Function");
    };
    let ty = f.params[0].ty.as_ref().expect("param type");
    match &ty.kind {
        TypeKind::Generic { args, .. } => assert_eq!(args.len(), 2),
        other => panic!("expected Pair<T, int>, got {other:?}"),
    }
}

#[test]
fn free_type_names_parse_without_binder() {
    let src = r#"
type Pair = {
    left: A
    right: B
}

id(x: T) = x
shape Boxy = {
    value: T
}
trait Wrapper = {
    unwrap(self) -> T
}
"#;
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("free names should parse");
    let Item::TypeDef(td) = &file.items[0] else {
        panic!("expected TypeDef");
    };
    assert!(
        td.generics.is_empty(),
        "binder is inferred later, not at parse"
    );
    let Item::Function(f) = &file.items[1] else {
        panic!("expected Function");
    };
    assert!(f.generics.is_empty());
    let Item::ShapeDef(s) = &file.items[2] else {
        panic!("expected ShapeDef");
    };
    assert!(s.generics.is_empty());
    let Item::TraitDef(t) = &file.items[3] else {
        panic!("expected TraitDef");
    };
    assert!(t.generics.is_empty());
}

#[test]
fn impl_without_trait_args_still_parses() {
    let src = r#"
impl Show for IntBox = {
    show(self) = "n"
}
"#;
    let mut p = Parser::new(src).expect("lex");
    let file = p.parse_file().expect("non-generic impl should parse");
    let Item::Impl(ib) = &file.items[0] else {
        panic!("expected Impl");
    };
    assert!(ib.trait_args.is_empty());
}
