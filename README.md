# Crisp

A systems programming language that transpiles to Rust. Write compact source; `crpc` infers types, ownership, lifetimes, and error propagation, emits Rust, and `rustc` is the soundness boundary.

**Spec:** [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)  
**Roadmap:** [ROADMAP.md](ROADMAP.md)

## Status

**v0.9.0** — stdlib shims (`vec`, `fs`, `sync`), `match` / `if`, `async`/`await` + tokio, `extern "C"` + `unsafe`, fallible `catch` emit fixed.

## Quick start

```bash
cargo build --release -p crpc
./target/release/crpc emit examples/hello
./target/release/crpc build examples/hello
./target/release/crpc run examples/hello
./target/release/crpc run examples/ffi
./target/release/crpc test examples/stdlib_smoke
./target/release/reveal rust examples/hello
cargo test --verbose
```

## Repository layout

```
crates/          Rust compiler workspace (crpc pipeline)
docs/spec/       Language specification
examples/        Sample .crp projects (hello, server, math, defaults, sealed, …)
std/             Standard library (Crisp)
tests/           Integration and compile-fail fixtures
```

## Philosophy

> Explicit on demand, implicit by default.

Crisp lowers to a typed IR (CIR), emits explicit Rust, and delegates memory safety to `rustc`. The `reveal` toolchain surfaces everything inference computed.
