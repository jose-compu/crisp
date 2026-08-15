//! User-facing generics + parametric shapes (#70 / #71).

use crisp_typeck::{TypeChecker, format_sig};
use std::path::PathBuf;

fn generics_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/generics")
}

#[test]
fn generics_example_typechecks() {
    let typed = TypeChecker::check_crate(&generics_root()).expect("typecheck generics");
    let mut names: Vec<_> = typed.signatures.keys().cloned().collect();
    names.sort();
    assert!(
        names.iter().any(|k| k.ends_with("::id")),
        "expected id signature, got {names:?}"
    );
    assert!(
        names.iter().any(|k| k.ends_with("::first")),
        "expected first signature, got {names:?}"
    );
    assert!(
        names.iter().any(|k| k.ends_with("::unwrap_int")),
        "expected unwrap_int signature, got {names:?}"
    );
    let id = typed
        .signatures
        .values()
        .find(|s| s.name == "id")
        .expect("id");
    eprintln!("id: {}", format_sig(id));
}
