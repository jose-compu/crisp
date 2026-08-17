use crisp_typeck::{TypeChecker, format_sig};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn import_from_module_sorting_after_main() {
    TypeChecker::check_crate(&fixture("module_order_after_main"))
        .expect("zutil sorts after main but must still typecheck (#13)");
}

#[test]
fn nested_caller_before_callee_infers_concrete_float() {
    let typed = TypeChecker::check_crate(&fixture("caller_before_callee"))
        .expect("twice(x) = scale(x, 2.0) should infer float when double sorts before scale");
    let twice = typed
        .signatures
        .values()
        .find(|s| s.name == "twice")
        .expect("twice");
    let shown = format_sig(twice);
    eprintln!("twice: {shown}");
    assert_eq!(shown, "twice(x: float) -> float");
    assert!(
        twice.generics.is_empty(),
        "must not publish twice<T>: {shown}"
    );
}
