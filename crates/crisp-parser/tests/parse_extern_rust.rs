use crisp_ast::item::Item;
use crisp_parser::Parser;

#[test]
fn parse_extern_rust_block() {
    let src = r#"
extern rust combustion_kernels {
    lap3(a: float, b: float, c: float, d: float) -> float
    load(path: str) -> str !
}
"#;
    let file = Parser::new(src).unwrap().parse_file().unwrap();
    let Item::Extern(ext) = &file.items[0] else {
        panic!("expected extern");
    };
    assert_eq!(ext.abi, "rust");
    assert_eq!(ext.rust_crate.as_ref().unwrap().name, "combustion_kernels");
    assert_eq!(ext.functions.len(), 2);
    assert_eq!(ext.functions[0].name.name, "lap3");
    assert!(!ext.functions[0].fallible);
    assert_eq!(ext.functions[1].name.name, "load");
    assert!(ext.functions[1].fallible);
}

#[test]
fn parse_extern_c_unchanged() {
    let src = r#"extern "C" { abs(x: int) -> int }"#;
    let file = Parser::new(src).unwrap().parse_file().unwrap();
    let Item::Extern(ext) = &file.items[0] else {
        panic!("expected extern");
    };
    assert_eq!(ext.abi, "C");
    assert!(ext.rust_crate.is_none());
    assert!(!ext.functions[0].fallible);
}
