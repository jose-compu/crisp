# Crisp

A systems programming language that transpiles to Rust. Write compact source; `crpc` infers types, ownership, lifetimes, and error propagation, emits Rust, and `rustc` is the soundness boundary.

**Spec:** [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)  
**Roadmap:** [ROADMAP.md](ROADMAP.md)

## Status

**v1.1.0** — Vec emit fixes (`push`/`len`/`mut`), complex examples (`inventory`, `workshop`, `vec_ops`, `fallible_chain`, `async_spawn`, `unsafe_math`, `data_pipeline`), CI clippy hardening.

## Quick start

```bash
cargo build --release -p crpc
./target/release/crpc emit examples/hello
./target/release/crpc build examples/hello
./target/release/crpc run examples/hello
./target/release/crpc run examples/ffi
./target/release/crpc run examples/kitchen_sink
./target/release/crpc run examples/inventory
./target/release/crpc test examples/workshop
./target/release/crpc run examples/vec_ops
./target/release/reveal rust examples/hello
cargo test --verbose
```

## Repository layout

```
crates/          Rust compiler workspace (crpc pipeline)
docs/spec/       Language specification
examples/        Sample .crp projects (hello, server, inventory, math, …)
std/             Standard library (Crisp)
tests/           Integration and compile-fail fixtures
```

## Philosophy

> Explicit on demand, implicit by default.

Crisp lowers to a typed IR (CIR), emits explicit Rust, and delegates memory safety to `rustc`. The `reveal` toolchain surfaces everything inference computed.
