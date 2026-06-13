use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crisp_cir::CirBuilder;
use crisp_rust_emit::emit_crate;
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn bench_emit_kitchen_sink(c: &mut Criterion) {
    let root = examples_dir().join("kitchen_sink");
    let cir = CirBuilder::build_crate(&root).expect("cir");
    c.bench_function("emit_kitchen_sink", |b| {
        b.iter(|| {
            let out = emit_crate(black_box(&cir));
            black_box(out.lib_rs.len());
        });
    });
}

fn bench_build_cir_server(c: &mut Criterion) {
    let root = examples_dir().join("server");
    c.bench_function("build_cir_server", |b| {
        b.iter(|| {
            CirBuilder::build_crate(black_box(&root)).expect("cir");
        });
    });
}

criterion_group!(benches, bench_emit_kitchen_sink, bench_build_cir_server);
criterion_main!(benches);
