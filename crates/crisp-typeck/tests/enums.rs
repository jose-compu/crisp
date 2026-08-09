use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn enums_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/enums")
}

#[test]
fn enums_example_typechecks() {
    TypeChecker::check_crate(&enums_root()).expect("enums typecheck");
}
