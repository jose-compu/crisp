//! Checking-position int → float and postfix `as` (#112).

use crisp_typeck::{TypeChecker, TypeError, TypeWarning, format_sig};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn issue_112_int_widens_in_float_slot() {
    let typed = TypeChecker::check_crate(&fixture("numeric_widen"))
        .expect("int in a float slot should typeck (#112)");
    let dx = typed
        .signatures
        .values()
        .find(|s| s.name == "dx_of")
        .expect("dx_of");
    assert_eq!(format_sig(dx), "dx_of(g: Grid1d) -> float");
    let use_scale = typed
        .signatures
        .values()
        .find(|s| s.name == "use_scale")
        .expect("use_scale");
    assert_eq!(format_sig(use_scale), "use_scale() -> float");
    let use_id = typed
        .signatures
        .values()
        .find(|s| s.name == "use_id")
        .expect("use_id");
    assert_eq!(
        format_sig(use_id),
        "use_id() -> int",
        "unconstrained id(1) must stay int"
    );
    assert!(
        typed.coercions.iter().any(|c| c.to_float && c.literal),
        "literal 1 in scale(1) should default as float: {:?}",
        typed.coercions
    );
    assert!(
        typed
            .warnings
            .iter()
            .any(|w| matches!(w, TypeWarning::IntToFloat { .. })),
        "non-literal g.nx → float should lint W0087: {:?}",
        typed.warnings
    );
}

#[test]
fn issue_112_explicit_as_silences_lint() {
    let typed = TypeChecker::check_crate(&fixture("numeric_widen")).expect("as float");
    let to_f = typed
        .signatures
        .values()
        .find(|s| s.name == "to_f")
        .expect("to_f");
    assert_eq!(format_sig(to_f), "to_f(n: int) -> float");
    let to_i = typed
        .signatures
        .values()
        .find(|s| s.name == "to_i")
        .expect("to_i");
    assert_eq!(format_sig(to_i), "to_i(x: float) -> int");
    assert!(
        typed.coercions.iter().any(|c| c.explicit && c.to_float),
        "n as float should record an explicit coercion"
    );
}

#[test]
fn issue_112_invalid_cast_is_e0087() {
    let err = TypeChecker::check_crate(&fixture("invalid_cast")).expect_err("bool as float");
    let msg = err.to_string();
    assert!(
        matches!(err, TypeError::InvalidCast { .. }) || msg.contains("E0087"),
        "got {err}"
    );
}
