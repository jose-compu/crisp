use crisp_ast::item::Item;
use crisp_parser::Parser;

#[test]
fn parse_bare_use_crate_like_typescript() {
    let src = "use serde_json { from_str, to_string }\n";
    let file = Parser::new(src).unwrap().parse_file().unwrap();
    let Item::Use(u) = &file.items[0] else {
        panic!("expected use");
    };
    assert_eq!(
        u.path.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["serde_json"]
    );
    let imports = u.imports.as_ref().unwrap();
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].name.name, "from_str");
    assert_eq!(imports[1].name.name, "to_string");
}

#[test]
fn parse_use_rust_dot_path() {
    let src = "use rust.serde_json { from_str, to_string }\n";
    let file = Parser::new(src).unwrap().parse_file().unwrap();
    let Item::Use(u) = &file.items[0] else {
        panic!("expected use");
    };
    assert_eq!(
        u.path.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["rust", "serde_json"]
    );
    let imports = u.imports.as_ref().unwrap();
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].name.name, "from_str");
    assert_eq!(imports[1].name.name, "to_string");
}

#[test]
fn parse_use_rust_colon_colon_path() {
    let src = "use rust::serde_json { from_str }\n";
    let file = Parser::new(src).unwrap().parse_file().unwrap();
    let Item::Use(u) = &file.items[0] else {
        panic!("expected use");
    };
    assert_eq!(
        u.path.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["rust", "serde_json"]
    );
}

#[test]
fn parse_use_rust_with_alias() {
    let src = "use rust.serde_json { Value as JsonValue }\n";
    let file = Parser::new(src).unwrap().parse_file().unwrap();
    let Item::Use(u) = &file.items[0] else {
        panic!("expected use");
    };
    let imports = u.imports.as_ref().unwrap();
    assert_eq!(imports[0].name.name, "Value");
    assert_eq!(imports[0].alias.as_ref().unwrap().name, "JsonValue");
}

#[test]
fn parse_mixed_bare_and_prefixed_uses() {
    let src = r#"
use serde_json { from_str }
use rust::serde_json { to_string }
"#;
    let file = Parser::new(src).unwrap().parse_file().unwrap();
    assert_eq!(file.items.len(), 2);
    let Item::Use(bare) = &file.items[0] else {
        panic!("bare");
    };
    let Item::Use(prefixed) = &file.items[1] else {
        panic!("prefixed");
    };
    assert_eq!(bare.path.len(), 1);
    assert_eq!(prefixed.path.len(), 2);
    assert_eq!(prefixed.path[0].name, "rust");
}
