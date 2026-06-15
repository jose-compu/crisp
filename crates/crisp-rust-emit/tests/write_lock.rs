//! Write committed crisp.lock for examples/sealed (run once: cargo test -p crisp-rust-emit write_sealed_lock -- --ignored --nocapture)

use crisp_rust_emit::update_lock;
use std::path::PathBuf;

#[test]
#[ignore]
fn write_sealed_lock() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/sealed");
    let lock = update_lock(&root).expect("update lock");
    println!(
        "wrote crisp.lock with {} sealed entries",
        lock.sealed_api.len()
    );
    for s in &lock.sealed_api {
        println!("  {}: {}", s.name, s.rust_signature);
    }
}
