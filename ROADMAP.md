# Crisp Roadmap

**Target spec:** [CrispLang-SPECS-0.2.0](docs/spec/CrispLang-SPECS-0.2.0.md)  
**Strategy:** Build the compiler in **Rust (v1)**, then rewrite it in **Crisp (v2)** and bootstrap.

---

## Vision

Crisp is a transpiler front end: `.crp` → CIR → Rust → native binary via `rustc`. The compiler infers HM types, global ownership modes, lifetimes, and ambient errors; emits explicit Rust; and treats any `rustc` failure on generated code as a `crisp` bug.

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

**CLI rename:** transpiler binary is now **`crisp`** (was `crispc`).

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

**Deliverable:** `crisp emit` + `crisp build` on `examples/hello`; runnable binary.

### Milestone 0.8 — Tooling & package management *(done — v0.8.0)*

**crisp commands** (§18.3)

- [x] `build`, `run`, `check`, `emit`, `test`

**reveal** (§16)

- [x] `types`, `ownership`, `lifetimes`, `errors`, `traits`
- [x] `rust`, `expand`, `diff`, `map`, `seal`

**Packages** (§18)

- [x] `crisp.toml` parser (`crisp-manifest`)
- [x] Dependency resolution (tokio + manifest deps → Cargo.toml)
- [x] Sealed-crate `crisp.lock` + signature drift detection (§12.5)

**Deliverable:** `examples/with_tests` runs `crisp test` (runtime + compile-fail); sealed API lockfile verified on build.

### Milestone 0.9 — Standard library & advanced features *(shipped in v0.9.0)*

**Std** (§15)

- [x] Core: vec shims (`new` / `push` / `len`); option/result/string/map/set symbols
- [x] IO: `std.fs.read_to_string` shim
- [x] IO: net; http via manifest deps (`std.net.parse_ip`, `ureq` / #28)
- [x] Concurrency: async/tokio (`#[tokio::main]`, `sleep_ms`)
- [ ] Concurrency: sync, atomic (symbols only)
- [x] Trait shims: Show, Eq, Ord (`examples/std_traits` / #27)

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
- [x] Stdio LSP host — `crisp-lsp` binary (#56, v1.5.0)

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
| `crisp` | CLI (`crisp`, `reveal`) | §18.3 |

---

## Versioning policy

- **Compiler** (`crisp`): `0.x` until v1.0 spec conformance; then semver.
- **Language** (`edition` in `crisp.toml`): `2026` for v0.2.0 spec; breaking changes bump edition.
- **Lockfile** (`crisp.lock`): regenerated on any `pub` API signature change (§12.5).

---

## How to contribute

1. Pick an unchecked item from the current milestone **or** an open publication issue (below).
2. Add tests under `tests/` before or with the implementation.
3. Cross-reference the spec section in PR description.
4. Run `cargo test` and `cargo clippy` locally.

---

## Publication readiness (first public release)

**v1.5.0** is the first public release track. Flip visibility when ready ([#58](https://github.com/jose-compu/crisp/issues/58)).

Current compiler: **v1.7.3**. Active milestones:

| Milestone | Semver | Focus | Board |
|-----------|--------|--------|-------|
| **v1.4.x** | patch/minor | Interop, traits, Show/Eq/Ord, net/http, preview hygiene (shipped) | [v1.4.0](https://github.com/jose-compu/crisp/milestone/5) · [v1.4.1](https://github.com/jose-compu/crisp/milestone/7) |
| **v1.5.0** | minor | Public launch: Result `?`, data shapes, stdio LSP, VSIX, trait defaults | [milestone](https://github.com/jose-compu/crisp/milestone/6) · label `release:v1.5.0` |
| **v1.6.0** | minor | Generics (implicit preferred), parametric shapes, pub schemes | label `release:v1.6.0` |
| **v1.6.1** | patch | Inferred bounds on `T`; E0084 on unsatisfied instantiations | label `release:v1.6.1` |
| **v1.7.0** | minor | First-class closures / function values (#72) | label `release:v1.7.0` |
| **v1.7.1** | patch | Nested emit, float powf, interpolation spans, while/if assign | label `release:v1.7.1` |
| **v1.7.2** | patch | Binop parens, nested type paths, string match, test harness | label `release:v1.7.2` |
| **v1.7.3** | patch | `crisp.toml` path deps; `crisp run` cwd = crate root | label `release:v1.7.3` |
| **v1.8.0** | minor | Parser DX, numeric widening, implicit `vec<T>`, math / `extern rust` | label `release:v1.8.0` · epics [#122](https://github.com/jose-compu/crisp/issues/122)–[#126](https://github.com/jose-compu/crisp/issues/126), [#110](https://github.com/jose-compu/crisp/issues/110) |
| **v2.0.0** | major | Compiler-as-library + self-hosting (Phase 2) | [milestone](https://github.com/jose-compu/crisp/milestone/4) |

| Priority | Theme | Examples |
|----------|--------|----------|
| **P0** | Public flip | [#58](https://github.com/jose-compu/crisp/issues/58) |
| **P1** | Trait bounds / `dyn` polish | [#59](https://github.com/jose-compu/crisp/issues/59) (defaults landed; bounds/`dyn` remain) |
| **P2** | Marketplace listing, channels / self-hosting | [#57](https://github.com/jose-compu/crisp/issues/57) (VSIX script landed), [#38](https://github.com/jose-compu/crisp/issues/38), [#30](https://github.com/jose-compu/crisp/issues/30)–[#32](https://github.com/jose-compu/crisp/issues/32) |
| **P1** | crates.io publish (v1.7.3) | [#66](https://github.com/jose-compu/crisp/issues/66) ([CRATES_IO.md](docs/CRATES_IO.md)) |
| **P2** | v1.7 language: first-class closures | [#72](https://github.com/jose-compu/crisp/issues/72) (shipped in v1.7.0) |

Filter: [issues with `epic:publication`](https://github.com/jose-compu/crisp/issues?q=is%3Aissue+is%3Aopen+label%3Aepic%3Apublication).

---

### In progress — v1.8.0

Epics: [#122](https://github.com/jose-compu/crisp/issues/122) parser/DX · [#123](https://github.com/jose-compu/crisp/issues/123) numeric typeck · [#124](https://github.com/jose-compu/crisp/issues/124) ownership · [#110](https://github.com/jose-compu/crisp/issues/110) collections · [#125](https://github.com/jose-compu/crisp/issues/125) stdlib/interop · [#126](https://github.com/jose-compu/crisp/issues/126) release.

- [x] **`else if`** — chained then-form and brace-form ([#117](https://github.com/jose-compu/crisp/issues/117))
- [x] **Record commas** — optional `,` in type/shape/literal fields ([#111](https://github.com/jose-compu/crisp/issues/111))
- [x] **Parse/lex `file:line:col`** — snippets instead of byte offsets ([#109](https://github.com/jose-compu/crisp/issues/109))
- [x] **Unary minus** — float `Neg`, CIR `Unary`, harness parenthesizes `assert_eq` RHS ([#113](https://github.com/jose-compu/crisp/issues/113))
- [x] **int → float** — checking-position widening, `as float` / `as int`, W0087, reveal coercions ([#112](https://github.com/jose-compu/crisp/issues/112))
- [x] **Record `:=`** — `Copy` on all-Copy fields; clone-at-bind when the source is reused ([#118](https://github.com/jose-compu/crisp/issues/118))
- [ ] **Test harness `&`** — call args from CIR ownership, not an AST heuristic ([#114](https://github.com/jose-compu/crisp/issues/114))

GitHub follow-up when the API recovers:

- Label `release:v1.8.0` on remaining story issues and comment `Child of Epic N`

---

### Shipped — v1.7.3 (path deps + run cwd)

- [x] **`crisp.toml` path deps** — `foo = { path = "…" }` emits a rewritten Cargo path into `target/rust/` ([#105](https://github.com/jose-compu/crisp/issues/105)); `examples/path_dep`
- [x] **`crisp run` cwd** — cargo `--manifest-path` with cwd = crate root; `CRISP_CRATE_ROOT` ([#106](https://github.com/jose-compu/crisp/issues/106))

### Shipped — v1.7.2 (compiler bugfixes)

- [x] **Binop grouping** — `(lo + hi) / 2` keeps parentheses ([#99](https://github.com/jose-compu/crisp/issues/99))
- [x] **Nested type `use`** — `crate::fail::a::Verdict` ([#100](https://github.com/jose-compu/crisp/issues/100)); `examples/nested_types`
- [x] **String `match`** — literal arms kept, not `_` ([#101](https://github.com/jose-compu/crisp/issues/101))
- [x] **`crisp test` harness** — unique names, bool/str `assert_eq!`, `pub trait Show` ([#102](https://github.com/jose-compu/crisp/issues/102))

### Shipped — v1.7.1 (compiler bugfixes)

- [x] **Nested `use` paths** — emit `crate::math::…` (`E0433`, [#93](https://github.com/jose-compu/crisp/issues/93)); unannotated `twice(x) = scale(x, 2.0)` infers across module order
- [x] **Float `**`** — `_f64` / `as f64` so `.powf` is not called on `{float}` (`E0689`, [#94](https://github.com/jose-compu/crisp/issues/94))
- [x] **Interpolation spans** — E0035 points at the string, not the first `use` ([#95](https://github.com/jose-compu/crisp/issues/95))
- [x] **while / if assign** — typeck Unit + CIR lower; `while {…}` is not called as a function ([#96](https://github.com/jose-compu/crisp/issues/96))

### Shipped — v1.7.0 (function values)

One callable kind. Named items and `|x| …` are the same values. Implicit sugar when a function is expected.

- [x] **Function values / closures** (spec §5.2–§5.3) — CIR + emit; `examples/closures` — [#72](https://github.com/jose-compu/crisp/issues/72)
- [x] **Holes** — `_ * 2` left-to-right; E0085 / E0086 — [#87](https://github.com/jose-compu/crisp/issues/87)
- [x] **Trailing last-arg** — `run { |x| … }` — [#88](https://github.com/jose-compu/crisp/issues/88)
- [x] **Point-free sections** — `.name`, `.magnitude()`, `.scale(2.0)` (baked extra args; not a two-arg function) — [#89](https://github.com/jose-compu/crisp/issues/89)

**Design note (#72, thanks [@aurelianito](https://github.com/aurelianito)):** Crisp should have **only closures** (first-class function values). Named “functions” are the same values bound to a name when useful — not a Ruby-style function-vs-closure split, and not a Java anonymous-inner-class escape hatch. Capture is lexical; ownership decides borrow/`move`. Emit may lower top-level named bindings to Rust `fn` and locals to Rust closures as a specialization, not as two language concepts.

Remaining after this milestone: operator sections (`+ 1`) by default out; `dyn Fn` with other `dyn` work (#59).

### Shipped — v1.6.1 (bound instantiation)

Keep parametric `T`. Infer constraints from the body. Reject unsatisfied instantiations in typeck.

- [x] **Prelude arith bounds** — `+` `-` `*` `/` on `T` → `Add`/`Sub`/`Mul`/`Div` — [#84](https://github.com/jose-compu/crisp/issues/84)
- [x] **Unique method → trait bound** — `x.show()` → `T: Show`; user `Measure` on `HasPosition<T>` (`examples/shapes_user`)
- [x] **E0084** — typeck rejects instantiations that do not satisfy inferred bounds
- [x] **`crisp test` several paths** — ignore a pasted `and` / repeated `crisp test`

### Shipped — v1.6.0 (generics)

Prefer implicit binders (`id(x: T)`, `type Pair = { left: A, right: B }`). `<>` remains a pin and is used for applications.

- [x] **User-facing generics** — types / functions / traits end-to-end — [#71](https://github.com/jose-compu/crisp/issues/71)
- [x] **Parametric shapes** — `shape Name = { value: T }` / `shape Name<T>` — [#70](https://github.com/jose-compu/crisp/issues/70)
- [x] **Implicit binders** — unbound type names are parameters — [#75](https://github.com/jose-compu/crisp/issues/75)
- [x] **Impl inference + `+` bounds** — [#77](https://github.com/jose-compu/crisp/issues/77)
- [x] **Publication schemes** — internal mono vs `pub` lock — [#76](https://github.com/jose-compu/crisp/issues/76)
- [x] **Defaults** — value restriction, reveal `T: Clone`, E0080 — [#78](https://github.com/jose-compu/crisp/issues/78)

---

*Last updated: 2026-08-17 — v1.8.0 in progress (test harness #114).*
