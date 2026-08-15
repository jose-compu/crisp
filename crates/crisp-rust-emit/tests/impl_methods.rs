//! Emit coverage for inherent impl methods (#20) + nested modules (#35).

use crisp_cir::CirBuilder;
use crisp_rust_emit::{emit_crate, run_emitted, run_tests};
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
}

#[test]
fn vec2_methods_emits_impl_and_nested_mod() {
    let cir = CirBuilder::build_crate(&example("vec2_methods")).expect("cir");
    let math_vector = cir
        .modules
        .iter()
        .find(|m| m.path == "math.vector")
        .expect("math.vector module");
    let impl_item = math_vector
        .items
        .iter()
        .find_map(|i| match i {
            crisp_cir::CirItem::Impl(ib) => Some(ib),
            _ => None,
        })
        .expect("CirImpl");
    assert_eq!(impl_item.ty_name, "Vec2");
    assert!(
        impl_item.functions.len() >= 3,
        "expected new/magnitude/scale, got {}",
        impl_item.functions.len()
    );

    let out = emit_crate(&cir);
    let vector_rs = out
        .modules
        .iter()
        .find(|(p, _)| p == "math.vector")
        .map(|(_, s)| s.as_str())
        .expect("emitted math.vector");
    assert!(vector_rs.contains("impl Vec2"));
    assert!(vector_rs.contains("pub fn new"));
    assert!(vector_rs.contains("pub fn magnitude(&self)"));
    assert!(out.lib_rs.contains("mod math"));
    assert!(out.lib_rs.contains(".magnitude()"));
}

#[test]
fn feature_gallery_emits_enum_beside_nested_mods() {
    let cir = CirBuilder::build_crate(&example("feature_gallery")).expect("cir");
    let out = emit_crate(&cir);
    assert!(
        out.lib_rs.contains("enum Color"),
        "main module must emit Color when nested mods exist:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("impl") || out.modules.iter().any(|(_, s)| s.contains("impl Point"))
    );
}

#[test]
fn vec2_methods_run_and_test() {
    let out = run_emitted(&example("vec2_methods")).expect("run");
    assert!(out.contains("m=5"), "stdout: {out}");
    let r = run_tests(&example("vec2_methods")).expect("test");
    assert!(r.runtime_passed >= 2);
}

#[test]
fn show_trait_emits_trait_and_impl() {
    let cir = CirBuilder::build_crate(&example("show_trait")).expect("cir");
    let main = cir.modules.iter().find(|m| m.path == "main").expect("main");
    assert!(
        main.items
            .iter()
            .any(|i| matches!(i, crisp_cir::CirItem::Trait(t) if t.name == "Show")),
        "expected CirTrait Show"
    );
    let trait_impl = main
        .items
        .iter()
        .find_map(|i| match i {
            crisp_cir::CirItem::Impl(ib) if ib.trait_name.as_deref() == Some("Show") => Some(ib),
            _ => None,
        })
        .expect("impl Show for Point");
    assert_eq!(trait_impl.ty_name, "Point");
    assert_eq!(trait_impl.functions.len(), 1);

    let out = emit_crate(&cir);
    assert!(
        out.lib_rs.contains("trait Show"),
        "emitted:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("impl Show for Point"),
        "emitted:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("fn show(&self)"),
        "emitted:\n{}",
        out.lib_rs
    );
    assert!(
        out.lib_rs.contains("fn label<T: Clone + Show>")
            && !out.lib_rs.contains("fn label<T: Clone + Copy + Show>"),
        "inferred T: Show without Copy:\n{}",
        out.lib_rs
    );
}

#[test]
fn show_trait_run_and_test() {
    let out = run_emitted(&example("show_trait")).expect("run");
    assert!(out.contains("p=(3,4)"), "stdout: {out}");
    assert!(out.contains("l=(3,4)"), "stdout: {out}");
    let r = run_tests(&example("show_trait")).expect("test");
    assert!(r.runtime_passed >= 2);
}
