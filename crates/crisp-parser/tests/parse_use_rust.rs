use crisp_ast::item::Item;
use crisp_parser::Parser;

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
