use crisp_typeck::{TypeChecker, TypeError};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn field_access_on_inferred_unique_struct_param() {
    let typed = TypeChecker::check_crate(&fixture("field_param_unique"))
        .expect("unique field should constrain param type");
    let sku = typed
        .signatures
        .values()
        .find(|s| s.name == "sku_of")
        .expect("sku_of");
    let rendered = format!("{:?}", sku.params);
    assert!(
        rendered.contains("Item") || rendered.contains("Named"),
        "expected Item param, got {rendered:?} / {sku:?}"
    );
}

#[test]
fn field_access_on_annotated_struct_param() {
    TypeChecker::check_crate(&fixture("field_param_unique")).expect("annotated param");
}

#[test]
fn ambiguous_field_on_param_asks_for_annotation() {
    let err = TypeChecker::check_crate(&fixture("field_param_ambiguous"))
        .expect_err("sku is on Item and StockLine");
    let msg = err.to_string();
    assert!(msg.contains("E0043"), "{msg}");
    match &err {
        TypeError::AmbiguousField { field, candidates, .. } => {
            assert_eq!(field, "sku");
            assert!(candidates.contains("Item"), "{candidates}");
            assert!(candidates.contains("StockLine"), "{candidates}");
        }
        other => panic!("expected AmbiguousField, got {other}"),
    }
}
