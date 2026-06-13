# Crisp

A systems programming language that transpiles to Rust. Write compact source; `crpc` infers types, ownership, lifetimes, and error propagation, emits Rust, and `rustc` is the soundness boundary.

**Spec:** [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)  
**Roadmap:** [ROADMAP.md](ROADMAP.md)

## Status

**v0.4.0** — HM type inference + `reveal types` (milestone 0.4). Transpiler CLI: **`crpc`** (was `crispc`).

## Quick start

```bash
cargo build --release -p crpc
./target/release/crpc check examples/server
./target/release/reveal types examples/hello
cargo test -p crisp-typeck -- --nocapture
```

## Repository layout

```
crates/          Rust compiler workspace (crpc pipeline)
docs/spec/       Language specification
examples/        Sample .crp projects
std/             Standard library (Crisp)
tests/           Integration and compile-fail fixtures
```

## Philosophy

> Explicit on demand, implicit by default.

Crisp lowers to a typed IR (CIR), emits explicit Rust, and delegates memory safety to `rustc`. The `reveal` toolchain surfaces everything inference computed.
