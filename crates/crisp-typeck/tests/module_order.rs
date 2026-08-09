use crisp_typeck::TypeChecker;
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
