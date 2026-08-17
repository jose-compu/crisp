# The Crisp Programming Language

[![CI](https://github.com/jose-compu/crisp/actions/workflows/ci.yml/badge.svg)](https://github.com/jose-compu/crisp/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.6.1-0A66C2.svg)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/rustc-1.85%2B-orange.svg)](Cargo.toml)
[![Tests](https://img.shields.io/badge/tests-280-brightgreen.svg)](.github/workflows/ci.yml)
[![Spec](https://img.shields.io/badge/spec-v0.2.0--draft-lightgrey.svg)](docs/spec/CrispLang-SPECS-0.2.0.md)
[![Docs](https://img.shields.io/badge/docs-online-informational.svg)](https://crisp-lang.org/)

<p align="center">
  <img src="assets/crisp-logo-square.jpg" alt="Crisp logo" width="320" />
</p>

> **Explicit on demand, implicit by default.**

Crisp (`.crp`) is a systems language that transpiles to Rust. You write compact source; `crisp` infers types, ownership, and error propagation, emits Rust, and `rustc` is the soundness boundary.

**This is a Rust-hosted bootstrap compiler (v1.6.1) — public release track.** It is **not** self-hosted yet (ROADMAP Phase 2 / milestone v2.0.0). The language document remains **[spec v0.2.0-draft](docs/spec/CrispLang-SPECS-0.2.0.md)** — treat “spec-complete” claims cautiously; see [known limitations](docs/KNOWN_LIMITATIONS.md) and [spec ↔ impl deltas](docs/SPEC_IMPL_DELTA.md).

Known Rust `Result` APIs from `rust = true` deps lower to Crisp ambient errors (`CrispError::Thrown` + `?`) — see `examples/rust_import`, `examples/net_http` (#55).

**Spec:** [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)  
**Quickstart:** [QUICKSTART.md](QUICKSTART.md)  
**Web docs:** [crisp-lang.org](https://crisp-lang.org/) · branch [`docs`](https://github.com/jose-compu/crisp/tree/docs)  
**Roadmap:** [ROADMAP.md](ROADMAP.md)  
**Changelog:** [CHANGELOG.md](CHANGELOG.md)  
**Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)  
**Security:** [SECURITY.md](SECURITY.md)  
**Milestone:** [v1.6.1](https://github.com/jose-compu/crisp/milestones)

License: **MIT OR Apache-2.0** ([LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).

## Philosophy

Crisp is optimized for **writing and local reading**. Semantics are defined by **lowering to Rust**, not by a separate runtime or VM.

In practice that means:

- Types, borrows, lifetimes, and error sets are **inferred globally** when the source stays silent.
- When you need precision — public APIs, performance-sensitive paths, or ambiguous usage — you annotate, and the compiler treats those annotations as hard constraints.
- **`reveal`** is the “show your work” companion to `crisp`: it prints inferred types, ownership (`&` / `&mut`), lifetimes, error sets, traits, and the emitted Rust that compact `.crp` source leaves implicit. See [QUICKSTART §10](QUICKSTART.md#10-inspect-what-the-compiler-inferred-reveal).

Crisp is a **front end that produces Rust**. Rust remains the authority on memory safety and data races. Crisp’s own borrow/region passes exist to drive good diagnostics and to decide what to emit (`&`, `&mut`, owned, `.clone()` fallbacks); they are not claimed as an independent soundness boundary. If generated Rust fails to compile, that is a **`crisp` bug**, not a user error.

**Design goals:** native code via `rustc`; HM-style type inference; deterministic global ownership dataflow; ambient fallible functions lowered to a uniform `Result<T, CrispError>`; compact syntax; tooling-first ergonomics.

**Non-goals:** scripting semantics; a GC; a stable library ABI across separately compiled units; pretending Crisp’s checker replaces `rustc`; claiming full enum/trait/shape coverage while those remain incomplete.

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
| CLI | `crisp` | `check`, `emit`, `build`, `run`, `test` |
| Inspect | `reveal` | Companion CLI: inferred types/ownership/errors, traits, emitted Rust (QUICKSTART §10) |
| IDE | `crisp-lsp` | Stdio LSP host (hover, inlay hints, diagnostics) + `CrispAnalysis` API (#56) |

**Sealed crates (`crisp.lock`):** a crate’s `pub` API has fully resolved signatures frozen at publish time. Downstream code analyzes against the lockfile, not re-inferred internals — the explicit tradeoff for whole-program inference inside a boundary.

**Type vs ownership inference:** HM-style unification handles types; ownership is a **separate deterministic dataflow pass** over the typed program. They are not unified — affine ownership does not compose with HM unification.

## Language (summary)

| Area | Surface | Lowers to |
|------|---------|-----------|
| Functions | named `f(x) = …` or `|x| …`; holes / trailing last-arg / `.field` | Rust `fn` or `move` closure / `impl Fn` |
| Bindings | `x := value` | `let` / `let mut` |
| Types | `type T = { … }`, `float`, `int`, `str`, … | Rust structs / aliases / `f64` / `i64` / `String` |
| Generics | Prefer `id(x: T)`, `type Pair = { left: A, right: B }`; `<>` pins / applies | Rust type params (`T: Clone` on emit) |
| Errors | `f() ! E`, `throw`, `catch` | `Result<T, CrispError>` |
| Modules | one file = one module; `use m { f }` | generated `mod` tree |
| Tests | `test`, `test_compile_fail` | injected `#[test]` in emitted crate |
| Async / FFI | `async`, `await`, `extern "C"` | Tokio / `extern` blocks (see examples) |

Comments: `--` and nested `{- -}`. String interpolation: `"hello {name}"`. Exponentiation: `**` → `.powf()`.

## Status

**v1.6.1** — inferred bounds on generic `T` from use (#84). **Unreleased (v1.7.0):** first-class function values and implicit-closure sugar (`examples/closures`, #72 / #87–#89).

**Still open:** crates.io republish ([#66](https://github.com/jose-compu/crisp/issues/66)), repo visibility ([#58](https://github.com/jose-compu/crisp/issues/58)); trait bounds / `dyn Trait` remain partial ([#59](https://github.com/jose-compu/crisp/issues/59)). See [ROADMAP.md](ROADMAP.md).

**MSRV:** Rust **1.85** (`rust-version` in root `Cargo.toml`). CI runs Ubuntu + macOS on stable, plus an MSRV job.

## Quick start

Install the compiler from crates.io (puts `crisp` and `reveal` on your `PATH` via `~/.cargo/bin`):

```bash
cargo install crisp-lang --locked
crisp --version
```

You still need a Rust toolchain (**1.85+**) with `cargo` / `rustc` — Crisp lowers to a Cargo project and builds with `rustc`.

### Hello world (no clone)

```bash
mkdir -p hello/src && cd hello

cat > crisp.toml <<'EOF'
[package]
name = "hello"
version = "0.1.0"
edition = "2026"

[build]
target = "rust"
runtime = "tokio"
error_model = "enum"
EOF

cat > src/main.crp <<'EOF'
shape Named = {
    name: str
}

type Guest = {
    name: str = "world"
}

id(x: T) = x

greet(who: Named) = "hello {who.name}"

pub main() = {
    world := Guest {}
    print(id(greet(world)))
}
EOF

crisp run .
```

Expected output:

```
"hello world"
```

### From this repository

```bash
git clone https://github.com/jose-compu/crisp.git
cd crisp
# already installed via cargo install crisp-lang, or:
# cargo install --path crates/crpc --locked
crisp run examples/hello
```

Other commands on a project:

```bash
crisp check .                 # resolve + typecheck
crisp emit .                  # write Rust under target/rust/
crisp build .                 # emit + cargo build
```

Optional LSP:

```bash
cargo install crisp-lsp --locked
```

See [QUICKSTART.md](QUICKSTART.md) for project layout, modules, tests, and fallible functions. crates.io notes: [docs/CRATES_IO.md](docs/CRATES_IO.md) ([#66](https://github.com/jose-compu/crisp/issues/66)).

## Examples

| Example | Topics | Notes |
|---------|--------|--------|
| `hello`, `math`, `float_demo`, `enums` | Shapes, implicit generics, integers, floats, enum + match | `crisp test` |
| `show_trait`, `trait_defaults`, `shapes` | Traits, defaults, data shapes | `crisp test` |
| `generics_implicit`, `generics`, `generics_pub`, `shapes_generic`, `shapes_user` | Implicit binders (preferred), pins, pub schemes, `HasPosition` + native ops / user `Measure` | `crisp test` |
| `std_traits`, `rust_import`, `net_http` | Show/Eq/Ord, Rust crates, thin HTTP | |
| `defaults`, `inventory`, `server` | Struct defaults, domain modules, config | |
| `fallible`, `fallible_chain` | `!`, `throw`, `catch`, error chains | |
| `vec_ops`, `data_pipeline` | `vec` stdlib, fallible IO | |
| `patterns`, `match` | Pattern matching (literal-oriented today) | see limitations |
| `async_hello`, `async_spawn` | Async / Tokio | |
| `ffi`, `unsafe_math` | C FFI, `unsafe` | |
| `sealed` | `crisp.lock` sealed public API | |
| `kitchen_sink`, `ownership_demo` | Combined features | |
| `workshop` | Small multi-file workshop | `crisp test` |
| `design_patterns` | GoF-style multi-module patterns | `crisp test` / `check` |
| `abnormal_suite` | Compile-fail edge cases | typecheck / fail tests |

## Repository layout

```
crates/          Rust compiler workspace (lexer → emit, crisp, lsp, reveal)
docs/            Spec, limitations, error catalog, web site scaffold
examples/        Sample .crp projects
std/             Standard library (Crisp prelude and modules)
tests/           Integration placeholders (fixtures live under crates/)
```

## Contributing / building

```bash
cargo build --release -p crisp-lang
cargo test --workspace --verbose
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md). Spec conformance and e2e coverage live under `crates/crisp-rust-emit/tests/` and `crates/crpc/tests/`.
