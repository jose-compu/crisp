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

### Milestone 0.8 — Tooling & package management *(current)*

**crispc commands** (§18.3)

- [ ] `build`, `run`, `check`, `emit`, `test`

**reveal** (§16)

- [ ] `types`, `ownership`, `lifetimes`, `errors`, `traits`
- [ ] `rust`, `expand`, `diff`, `map`, `seal`

**Packages** (§18)

- [ ] `crisp.toml` parser
- [ ] Dependency resolution
- [ ] Sealed-crate `crisp.lock` + signature drift detection (§12.5)

**Deliverable:** full spec §20 server example builds end-to-end.

### Milestone 0.9 — Standard library & advanced features

**Std** (§15)

- [ ] Core: option, result, string, vec, map, set
- [ ] IO: fs, io, net; http via manifest deps
- [ ] Concurrency: async/tokio, sync, atomic
- [ ] Trait shims: Show, Eq, Ord, …

**Language features**

- [ ] Pattern matching (§10)
- [ ] Concurrency: spawn, async/await (§11)
- [ ] FFI `extern "C"` (§14)
- [ ] `unsafe` blocks (delegated to emitted Rust)
- [ ] `test` / `test_compile_fail` harness (§19)

**Deliverable:** stdlib smoke tests; FFI round-trip example.

### Milestone 1.0 — Editor & hardening

**LSP** (§16.3)

- [ ] Ghost-text type hints, hover, ownership overlays
- [ ] Reachable-error-set on calls
- [ ] "Show emitted Rust" code lens

**Quality**

- [ ] Fuzz lexer/parser
- [ ] End-to-end tests for every spec section
- [ ] Performance benchmarks (inference fixpoint, emit)
- [ ] User-facing error message polish (§17.4)

**Deliverable:** v1.0 release — spec-complete bootstrap compiler.

---

## Phase 2 — Self-hosted compiler (Crisp)

Prerequisite: **v1.0** passes a frozen conformance suite that v2 must reproduce bit-for-bit on emitted Rust (or within defined equivalence).

### Milestone 2.0 — Compiler-as-library

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

1. Pick an unchecked item from the current milestone.
2. Add tests under `tests/` before or with the implementation.
3. Cross-reference the spec section in PR description.
4. Run `cargo test` and `cargo clippy` locally.

---

*Last updated: 2026-06-12 — v0.7.0 CIR + Rust emission shipped; milestone 0.8 in progress.*
