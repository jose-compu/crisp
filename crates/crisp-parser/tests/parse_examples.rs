//! Parse all example projects.

use crisp_parser::Parser;
use std::fs;
use std::path::PathBuf;

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn parse_crate(name: &str) {
    let src = examples().join(name).join("src");
    for entry in fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|e| e.to_str()) != Some("crp") {
            continue;
        }
        let path = entry.path();
        let raw = fs::read_to_string(&path).unwrap();
        let mut parser = Parser::new(&raw).expect("parser");
        let ast = parser.parse_file().expect("parse");
        assert!(!ast.items.is_empty(), "{} empty", path.display());
        eprintln!("parsed {} ({} items)", path.display(), ast.items.len());
    }
}

#[test]
fn parse_all_examples() {
    for name in [
        "hello",
        "server",
        "fallible",
        "with_tests",
        "math",
        "defaults",
        "sealed",
    ] {
        eprintln!("=== {name} ===");
        parse_crate(name);
    }
}

#[test]
fn parse_math_has_tests_and_arith() {
    let main = fs::read_to_string(examples().join("math/src/main.crp")).unwrap();
    let arith = fs::read_to_string(examples().join("math/src/arith.crp")).unwrap();
    let mut p = Parser::new(&arith).unwrap();
    let ast = p.parse_file().unwrap();
    let tests = ast
        .items
        .iter()
        .filter(|i| matches!(i, crisp_ast::item::Item::Test(_)))
        .count();
    assert_eq!(tests, 3);
    assert!(main.contains("use arith"));
}
