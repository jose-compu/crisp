# Crisp Roadmap

**Target spec:** [CrispLang-SPECS-0.2.0](docs/spec/CrispLang-SPECS-0.2.0.md)  
**Strategy:** Build the compiler in **Rust (v1)**, then rewrite it in **Crisp (v2)** and bootstrap.

---

## Vision

Crisp is a transpiler front end: `.crp` → CIR → Rust → native binary via `rustc`. The compiler infers HM types, global ownership modes, lifetimes, and ambient errors; emits explicit Rust; and treats any `rustc` failure on generated code as a `crpc` bug.

Two deliverables define success:

| Version | Implementation | Goal |
|---------|----------------|------|
| **v1 (bootstrap)** | Rust workspace (`crates/`) | Full spec compliance; production-quality toolchain |
| **v2 (self-hosted)** | Crisp sources (`compiler/`) | `crispc` compiles itself; Rust implementation becomes fallback |

---

## Compiler pipeline (reference)

All milestones map to this pipeline from spec §17.1:

```
Lexer → Parser → Name Resolution → Type Inference → Ownership Pass
  → Region Pass → Error Pass → CIR → Rust Emission → rustc
```

Supporting systems: `reveal` (§16), sealed crates / `crisp.lock` (§12.5), `crisp.toml` (§18), LSP (§16.3).

---

## Phase 1 — Bootstrap compiler (Rust)

### Milestone 0.1 — Scaffold *(done — v0.1.0)*

- [x] Cargo workspace aligned to pipeline crates
- [x] `crispc` / `reveal` CLI stubs
- [x] Spec in `docs/spec/`
- [x] Example `.crp` projects
- [x] `cargo build` green across workspace
- [x] CI: `cargo test`, `cargo clippy`, `cargo fmt`

### Milestone 0.2 — Front end *(done — v0.2.0)*

**Lexer** (§2)

- [x] UTF-8, comments (`--`, `{- -}`)
- [x] Keywords, identifiers, raw-escape map for Rust keywords
- [x] Operators: `&&&`, `|||`, `^^^`, `:=`, `mut:=`, `|>`, postfix `?`
- [x] Literals: int, float, string interpolation, raw/multi-line strings, char

**Parser** (Appendix A)

- [x] Expression-based AST with brace blocks
- [x] Items: fn, type, trait, shape, impl, test, test_compile_fail
- [x] Control flow: if/match/for/while/loop, async/await
- [x] Diagnostic spans on every AST node

**Deliverable:** parse `examples/hello/src/main.crp` to AST; snapshot tests.

### Milestone 0.3 — Name resolution & modules *(done — v0.3.0)*

**Resolver** (§12)

- [x] File-based module tree (no `mod` in source)
- [x] `use` / `pub use` / `as` imports
- [x] Prelude injection (`std/prelude.crp`)
- [x] Symbol tables, visibility (`pub` / private)

**Deliverable:** multi-file `examples/server` resolves without errors.

### Milestone 0.4 — Type inference *(done — v0.4.0)*

**Typeck** (§3)

- [x] Primitives, tuples, arrays, slices, `vec`, `map`, `?T`
- [x] Structs, enums, aliases; recursive enum `Box` planning (structs)
- [x] HM inference + constraint solving
- [x] Shapes → generated trait plan (stub); traits → direct mapping (stub)
- [x] Generics, `where` clauses (partial)
- [x] Explicit annotations as hard constraints

**Deliverable:** `reveal types` on hello example; inference tests.

**CLI rename:** transpiler binary is now **`crpc`** (was `crispc`).

### Milestone 0.5 — Ownership & regions *(done — v0.5.0)*

**Ownership** (§7)

- [x] Usage collection: read / mutate / move-out / copy
- [x] Lattice join (`&` ⊑ `&mut` ⊑ owned) + global call-graph fixpoint
- [x] Clone insertion policy (reported via `reveal ownership`)
- [x] Explicit `&` / `&mut` / `own` annotations
- [x] rustc disagreement fallbacks (§7.6) — probe emit + rustc retry loop

**Regions** (§8)

- [x] Lifetime assignment; Rust elision where applicable
- [x] Explicit `'a` emission where needed

**Deliverable:** `reveal ownership` / `reveal lifetimes` on ownership test suite.

### Milestone 0.6 — Error handling *(done — v0.6.0)*

**Error pass** (§9)

- [x] Ambient fallibility → `Result<T, CrispError>`
- [x] Program-global `CrispError` enum synthesis
- [x] `catch`, `throw`, `!` error-set annotations (documentation only)
- [x] Reachable error-set analysis for `reveal errors`

**Deliverable:** `examples/fallible` + fallible probe emit through rustc.

### Milestone 0.7 — CIR & Rust emission *(done — v0.7.0)*

**CIR** (§17.1)

- [x] Typed IR nodes with ownership + span map
- [x] Shape-trait + impl synthesis
- [x] Default-field builder synthesis (§3.3.1)
- [x] Box insertion at recursion points
- [x] Clone/borrow/move materialization

**Rust emit** (§17.1, §17.3)

- [x] Pretty-print CIR → idiomatic Rust
- [x] Emit `target/rust/` Cargo project (§18)
- [x] Invoke `cargo build` / `cargo check`
- [x] Map `rustc` spans back to Crisp ICE diagnostics
- [x] §7.6 fallback rewrites (reborrow, clone, widen-mut) via probe emit

**Deliverable:** `crpc emit` + `crpc build` on `examples/hello`; runnable binary.

### Milestone 0.8 — Tooling & package management *(done — v0.8.0)*

**crpc commands** (§18.3)

- [x] `build`, `run`, `check`, `emit`, `test`

**reveal** (§16)

- [x] `types`, `ownership`, `lifetimes`, `errors`, `traits`
- [x] `rust`, `expand`, `diff`, `map`, `seal`

**Packages** (§18)

- [x] `crisp.toml` parser (`crisp-manifest`)
- [x] Dependency resolution (tokio + manifest deps → Cargo.toml)
- [x] Sealed-crate `crisp.lock` + signature drift detection (§12.5)

**Deliverable:** `examples/with_tests` runs `crpc test` (runtime + compile-fail); sealed API lockfile verified on build.

### Milestone 0.9 — Standard library & advanced features *(shipped in v0.9.0)*

**Std** (§15)

- [x] Core: vec shims (`new` / `push` / `len`); option/result/string/map/set symbols
- [x] IO: `std.fs.read_to_string` shim
- [ ] IO: net; http via manifest deps
- [x] Concurrency: async/tokio (`#[tokio::main]`, `sleep_ms`)
- [ ] Concurrency: sync, atomic (symbols only)
- [ ] Trait shims: Show, Eq, Ord, …

**Language features**

- [x] Pattern matching (§10) — CIR + emit; `examples/match`
- [x] Concurrency: spawn, async/await (§11) — CIR + emit; `examples/async_hello`
- [x] FFI `extern "C"` (§14) — `examples/ffi` (libc `abs` round-trip)
- [x] `unsafe` blocks (delegated to emitted Rust)
- [x] `test` / `test_compile_fail` harness (§19) — shipped in v0.8

**Deliverable:** `examples/stdlib_smoke` + `crates/crisp-rust-emit/tests/m09_features.rs`; FFI round-trip in `examples/ffi`.

### Milestone 1.0 — Editor & hardening *(shipped v1.0.0)*

**LSP** (§16.3)

- [x] Ghost-text type hints, hover, ownership overlays — `crisp-lsp` analysis API
- [x] Reachable-error-set on calls — `call_overlays`
- [x] "Show emitted Rust" code lens — `code_lenses` + `emitted_rust`

**Quality**

- [x] Fuzz lexer/parser — `crisp-parser/tests/fuzz_smoke.rs`
- [x] End-to-end tests for every spec section — `conformance_e2e.rs`, `m10_features.rs`
- [x] Performance benchmarks (inference fixpoint, emit) — criterion benches in `typeck` + `rust-emit`
- [x] User-facing error message polish (§17.4) — `crisp-diagnostics/format.rs`

**Deliverable:** v1.0 release — spec-complete bootstrap compiler.

### Milestone 1.1 — Emit fixes & complex examples *(shipped v1.1.0)*

- [x] Fix `Vec::push` / `Vec::len` / `let mut` lowering in `crisp-rust-emit`
- [x] Multi-module examples: `inventory`, `workshop`
- [x] Feature-matrix examples: `vec_ops`, `fallible_chain`, `async_spawn`, `unsafe_math`, `data_pipeline`
- [x] `m11_complex_features` e2e suite; expanded conformance/cli/lsp tests
- [x] CI: `cargo fmt --check`, `clippy -D warnings` clean
- [x] Spec v0.2 abnormal-path suite: `spec_abnormal_v2.rs`, `examples/abnormal_suite`
- [x] GoF-style patterns example: `examples/design_patterns` (multi-module)

**Deliverable:** v1.1 — correct vec lowering and broad complex-feature coverage.

### Milestone 2.0 — Compiler-as-library *(current)*

- [ ] Expose v1 pipeline as a stable internal API (crate boundaries documented)
- [ ] Define which crates move to Crisp vs stay in Rust (FFI shim, LSP host)
- [ ] Pin conformance corpus: parse → CIR → Rust snapshots

### Milestone 2.1 — Rewrite front end in Crisp

Port to `compiler/` in Crisp:

- [ ] Lexer
- [ ] Parser
- [ ] AST data structures

**Gate:** Crisp lexer/parser compile via v1 `crispc`; output matches v1 snapshots.

### Milestone 2.2 — Rewrite analysis passes in Crisp

- [ ] Name resolution
- [ ] Type inference
- [ ] Ownership + regions
- [ ] Error pass

**Gate:** `reveal` output identical to v1 on conformance corpus.

### Milestone 2.3 — Rewrite CIR & emission in Crisp

- [ ] CIR builder
- [ ] Rust pretty-printer
- [ ] `crisp.lock` / sealed crate logic

**Gate:** `crispc build` on `examples/` using Crisp-built `crispc` produces same `target/rust/` as v1.

### Milestone 2.4 — Bootstrap loop

```
crispc-v1  ──compiles──▶  crispc.crp  ──▶  crispc-v2
crispc-v2  ──compiles──▶  crispc.crp  ──▶  crispc-v3  (must equal v2)
```

- [ ] Fixpoint bootstrap verified in CI
- [ ] v1 demoted to `crispc-bootstrap` emergency fallback
- [ ] Document bootstrap procedure in repo

### Milestone 2.5 — v2 release

- [ ] Default `crispc` binary is self-hosted
- [ ] Rust sources archived or limited to runtime shim
- [ ] Performance parity with v1 (±10% on conformance builds)

**Deliverable:** v2.0 — Crisp compiles Crisp.

---

## Phase 3 — Beyond self-hosting (future)

Not committed to dates; tracked for direction only.

- Separate compilation research (spec currently forbids stable ABI — §0.2)
- Incremental / watch-mode compilation
- Crisp-written package registry client
- Alternative emit targets (still Rust-first per spec)
- Crisp playground / web REPL

---

## Crate map (v1 Rust workspace)

| Crate | Pipeline stage | Spec |
|-------|----------------|------|
| `crisp-lexer` | Lexer | §2 |
| `crisp-parser` | Parser | Appendix A |
| `crisp-ast` | AST | §17.1 |
| `crisp-resolve` | Name resolution | §12 |
| `crisp-typeck` | Type inference | §3.4 |
| `crisp-ownership` | Ownership pass | §7 |
| `crisp-regions` | Region pass | §8 |
| `crisp-errors` | Error pass | §9 |
| `crisp-cir` | CIR generation | §17.1 |
| `crisp-rust-emit` | Rust emission | §17.1 |
| `crisp-diagnostics` | Diagnostics | §17.2–17.4 |
| `crisp-manifest` | `crisp.toml` / `crisp.lock` | §12.5, §18 |
| `crisp-reveal` | reveal toolchain | §16 |
| `crisp-lsp` | LSP | §16.3 |
| `crpc` | CLI (`crpc`, `reveal`) | §18.3 |

---

## Versioning policy

- **Compiler** (`crpc`): `0.x` until v1.0 spec conformance; then semver.
- **Language** (`edition` in `crisp.toml`): `2026` for v0.2.0 spec; breaking changes bump edition.
- **Lockfile** (`crisp.lock`): regenerated on any `pub` API signature change (§12.5).

---

## How to contribute

1. Pick an unchecked item from the current milestone **or** an open publication issue (below).
2. Add tests under `tests/` before or with the implementation.
3. Cross-reference the spec section in PR description.
4. Run `cargo test` and `cargo clippy` locally.

---

## Publication readiness (public release backlog)

Tracking epic: [jose-compu/crisp#1](https://github.com/jose-compu/crisp/issues/1) (labels: `epic:publication`, `P0` / `P1` / `P2`).

Current compiler: **v1.3.0**. Planned GitHub milestones:

| Milestone | Semver | Focus | Board |
|-----------|--------|--------|-------|
| **v1.1.1** | patch | Docs, license, release hygiene, CI/example matrix | [milestone](https://github.com/jose-compu/crisp/milestone/1) · label `release:v1.1.1` |
| **v1.2.0** | minor | Critical language/DX (enums, fields, modules, probe) — first public-usable target | [milestone](https://github.com/jose-compu/crisp/milestone/2) · label `release:v1.2.0` |
| **v1.3.0** | minor | LSP docs, reveal polish, inherent impl methods, CI matrix (shipped) | [milestone](https://github.com/jose-compu/crisp/milestone/3) · label `release:v1.3.0` |
| **v1.4.0** | minor | Rust crate interop, `trait` / `impl Trait for`, stdlib expansion | [milestone](https://github.com/jose-compu/crisp/milestone/5) · label `release:v1.4.0` |
| **v2.0.0** | major | Compiler-as-library + self-hosting (Phase 2) | [milestone](https://github.com/jose-compu/crisp/milestone/4) · label `release:v2.0.0` |

| Priority | Theme | Examples |
|----------|--------|----------|
| **P0** | Docs, honesty, install, known limitations, critical compiler gaps | [#2](https://github.com/jose-compu/crisp/issues/2)–[#17](https://github.com/jose-compu/crisp/issues/17); also [#33](https://github.com/jose-compu/crisp/issues/33) license, [#34](https://github.com/jose-compu/crisp/issues/34) enums, [#36](https://github.com/jose-compu/crisp/issues/36) design_patterns e2e |
| **P1** | Interop, traits (#50), shapes, stdlib shims | [#24](https://github.com/jose-compu/crisp/issues/24)–[#29](https://github.com/jose-compu/crisp/issues/29), [#50](https://github.com/jose-compu/crisp/issues/50); also [#35](https://github.com/jose-compu/crisp/issues/35) nested mods, [#37](https://github.com/jose-compu/crisp/issues/37) parse coverage |
| **P2** | Deferred (watch mode, library API, self-hosting, channels) | [#30](https://github.com/jose-compu/crisp/issues/30)–[#32](https://github.com/jose-compu/crisp/issues/32), [#38](https://github.com/jose-compu/crisp/issues/38) |

Filter: [issues with `epic:publication`](https://github.com/jose-compu/crisp/issues?q=is%3Aissue+is%3Aopen+label%3Aepic%3Apublication).

---

*Last updated: 2026-08-12 — v1.3.0 shipped; trait path → #50 / v1.4.0.*
