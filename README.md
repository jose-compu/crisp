# Crisp

A systems programming language that transpiles to Rust. You write compact `.crp` source; `crpc` infers types, ownership, lifetimes, and error propagation, emits explicit Rust, and `rustc` is the soundness boundary.

**Spec:** [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)  
**Quickstart:** [QUICKSTART.md](QUICKSTART.md)  
**Roadmap:** [ROADMAP.md](ROADMAP.md)

## Philosophy

Crisp is optimized for **writing and local reading**. Semantics are defined by **lowering to Rust**, not by a separate runtime or VM.

> **Explicit on demand, implicit by default.**

In practice that means:

- Types, borrows, lifetimes, and error sets are **inferred globally** when the source stays silent.
- When you need precision — public APIs, performance-sensitive paths, or ambiguous usage — you annotate, and the compiler treats those annotations as hard constraints.
- **`reveal`** reconstructs everything inference computed: Rust signatures, ownership modes, lifetime overlays, and the error slice a function can produce.

Crisp is a **front end that produces Rust**. Rust remains the authority on memory safety and data races. Crisp’s own borrow/region passes exist to drive good diagnostics and to decide what to emit (`&`, `&mut`, owned, `.clone()` fallbacks); they are not claimed as an independent soundness boundary. If generated Rust fails to compile, that is a **`crpc` bug**, not a user error.

**Design goals:** native code via `rustc`; HM-style type inference; deterministic global ownership dataflow; ambient fallible functions lowered to a uniform `Result<T, CrispError>`; compact syntax; tooling-first ergonomics.

**Non-goals:** scripting semantics; a GC; a stable library ABI across separately compiled units; pretending Crisp’s checker replaces `rustc`.

## Architecture

Crisp v0.2.0 (spec) retargets from “direct-to-LLVM language” to **source → CIR → Rust → native**. The unit of compilation is the **whole program or a sealed crate** — inferred signatures depend on call sites, so ad-hoc separate compilation without a lockfile is not supported.

```
.crp sources
    │
    ▼
Lexer → Parser → Resolve → Type inference → Ownership → Regions → Errors
    │
    ▼
CIR (typed, ownership-resolved IR)
    │
    ▼
Rust emission  ──►  rustc  ──►  native binary
    │
    └── probe emit + rustc (§7.6 fallbacks when ownership disagrees)
```

| Stage | Crate / tool | Role |
|-------|----------------|------|
| Lex / parse | `crisp-lexer`, `crisp-parser` | UTF-8 `.crp`, expression-based AST, spans |
| Resolve | `crisp-resolve` | File modules, `use`, prelude, visibility |
| Typeck | `crisp-typeck` | HM inference + constraint solving |
| Ownership | `crisp-ownership` | Global usage → `&` / `&mut` / owned; §7.6 fallbacks |
| Regions / errors | `crisp-regions`, `crisp-errors` | Lifetimes, ambient `!` → `CrispError` |
| IR | `crisp-cir` | Typed CIR consumed by emit |
| Emit | `crisp-rust-emit` | Rust project under `target/rust/`, tests, `crisp.lock` |
| CLI | `crpc` | `check`, `emit`, `build`, `run`, `test` |
| Inspect | `reveal` | Inferred Rust, ownership, errors, sealed API |
| IDE | `crisp-lsp` | Hover, hints, overlays on inferred precision |

**Sealed crates (`crisp.lock`):** a crate’s `pub` API has fully resolved signatures frozen at publish time. Downstream code analyzes against the lockfile, not re-inferred internals — the explicit tradeoff for whole-program inference inside a boundary.

**Type vs ownership inference:** HM-style unification handles types; ownership is a **separate deterministic dataflow pass** over the typed program. They are not unified — affine ownership does not compose with HM unification.

## Language (summary)

| Area | Surface | Lowers to |
|------|---------|-----------|
| Functions | `name(args) = expr` or `{ … }` block | `fn` with inferred/annotated params |
| Bindings | `x := value` | `let` / `let mut` |
| Types | `type T = { … }`, `float`, `int`, `str`, … | Rust structs / aliases / `f64` / `i64` / `String` |
| Errors | `f() ! E`, `throw`, `catch` | `Result<T, CrispError>` |
| Modules | one file = one module; `use m { f }` | generated `mod` tree |
| Tests | `test`, `test_compile_fail` | injected `#[test]` in emitted crate |
| Async / FFI | `async`, `await`, `extern "C"` | Tokio / `extern` blocks (see examples) |

Comments: `--` and nested `{- -}`. String interpolation: `"hello {name}"`. Exponentiation: `**` → `.powf()`.

## Status

**v1.1.0** — Vec emit fixes; expanded examples (`design_patterns`, `float_demo`, `abnormal_suite`); float literal/`**` emit; probe borrow-check fixes; LSP analysis API; spec v0.2 abnormal-path tests.

Milestone progress and remaining spec gaps: [ROADMAP.md](ROADMAP.md).

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
./target/release/crpc test examples/abnormal_suite
./target/release/crpc test examples/design_patterns
./target/release/crpc test examples/float_demo
./target/release/reveal rust examples/hello
cargo test --verbose
```

See [QUICKSTART.md](QUICKSTART.md) for project layout, modules, tests, and fallible functions.

## Examples

| Example | Topics |
|---------|--------|
| `hello`, `math`, `float_demo` | Basics, integers, floats, multi-module tests |
| `defaults`, `inventory`, `server` | Struct defaults, domain modules, config |
| `fallible`, `fallible_chain` | `!`, `throw`, `catch`, error chains |
| `vec_ops`, `data_pipeline` | `vec` stdlib, fallible IO |
| `patterns`, `match` | Pattern matching |
| `async_hello`, `async_spawn` | Async / Tokio |
| `ffi`, `unsafe_math` | C FFI, `unsafe` |
| `sealed` | `crisp.lock` sealed public API |
| `kitchen_sink` | Combined features |
| `design_patterns` | GoF-style multi-module patterns |
| `abnormal_suite` | Compile-fail edge cases (spec audit) |

## Repository layout

```
crates/          Rust compiler workspace (lexer → emit, crpc, lsp, reveal)
docs/spec/       Language specification (v0.2.0-draft)
examples/        Sample .crp projects
std/             Standard library (Crisp prelude and modules)
tests/           Integration and compile-fail fixtures
```

## Contributing / building

```bash
cargo build --release -p crpc -p reveal
cargo test --workspace --verbose
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
```

Spec conformance and e2e coverage live under `crates/crisp-rust-emit/tests/` and `crates/crpc/tests/`.
