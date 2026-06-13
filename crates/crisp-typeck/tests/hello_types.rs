use crisp_typeck::{TypeChecker, format_sig};
use std::path::PathBuf;

fn hello_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello")
}

#[test]
fn infer_hello_signatures() {
    let typed = TypeChecker::check_crate(&hello_root()).expect("typecheck hello");
    let mut lines: Vec<String> = typed.signatures.values().map(format_sig).collect();
    lines.sort();
    insta::assert_debug_snapshot!(lines);
}
