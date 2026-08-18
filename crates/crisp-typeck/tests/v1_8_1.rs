//! v1.8.1 typeck regressions.

use crisp_typeck::TypeChecker;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn issue_146_same_private_name_in_nested_modules() {
    TypeChecker::check_crate(&fixture("issue_146_rk4"))
        .expect("private rk4_k2 in core.a vs core.b must not unify (#146)");
}
