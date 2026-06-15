use crisp_typeck::TypeChecker;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn bench_typecheck_server(c: &mut Criterion) {
    let root = examples_dir().join("server");
    c.bench_function("typecheck_server", |b| {
        b.iter(|| {
            TypeChecker::check_crate(black_box(&root)).expect("typeck");
        });
    });
}

fn bench_typecheck_kitchen_sink(c: &mut Criterion) {
    let root = examples_dir().join("kitchen_sink");
    c.bench_function("typecheck_kitchen_sink", |b| {
        b.iter(|| {
            TypeChecker::check_crate(black_box(&root)).expect("typeck");
        });
    });
}

criterion_group!(
    benches,
    bench_typecheck_server,
    bench_typecheck_kitchen_sink
);
criterion_main!(benches);
