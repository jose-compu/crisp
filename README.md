# Crisp

A systems programming language that transpiles to Rust. Write compact source; `crispc` infers types, ownership, lifetimes, and error propagation, emits Rust, and `rustc` is the soundness boundary.

**Spec:** [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)  
**Roadmap:** [ROADMAP.md](ROADMAP.md)

## Status

**v0.3.0** — name resolution + modules (milestone 0.3). Bootstrap compiler in Rust (v1); self-hosted Crisp (v2) planned.

## Quick start

```bash
cargo build --release
./target/release/crispc resolve examples/server
./target/release/crispc check examples/server
cargo test -p crisp-resolve -- --nocapture
```

## Repository layout

```
crates/          Rust compiler workspace (crispc pipeline)
docs/spec/       Language specification
examples/        Sample .crp projects
std/             Standard library (Crisp)
tests/           Integration and compile-fail fixtures
```

## Philosophy

> Explicit on demand, implicit by default.

Crisp lowers to a typed IR (CIR), emits explicit Rust, and delegates memory safety to `rustc`. The `reveal` toolchain surfaces everything inference computed.
