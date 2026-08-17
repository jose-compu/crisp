//! Unary minus on float stays float (#113).

use crisp_typeck::{TypeChecker, format_sig};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn issue_113_unary_minus_float_typecks() {
    let typed = TypeChecker::check_crate(&fixture("unary_neg_float"))
        .expect("-2.0 / -x on float should typeck (#113)");
    let lap3 = typed
        .signatures
        .values()
        .find(|s| s.name == "lap3")
        .expect("lap3");
    assert_eq!(
        format_sig(lap3),
        "lap3(um: float, uc: float, up: float, dx: float) -> float"
    );
    let negf = typed
        .signatures
        .values()
        .find(|s| s.name == "negf")
        .expect("negf");
    assert_eq!(format_sig(negf), "negf(x: float) -> float");
    let negi = typed
        .signatures
        .values()
        .find(|s| s.name == "negi")
        .expect("negi");
    assert_eq!(format_sig(negi), "negi(n: int) -> int");
}
