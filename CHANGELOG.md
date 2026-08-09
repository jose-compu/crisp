# Changelog

All notable changes to the Crisp toolchain (`crpc`, `reveal`, and workspace crates) are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

Language edition (`crisp.toml` `edition = "2026"`) tracks the abstract language design; compiler versions below are independent.

## [Unreleased]

## [1.2.0] — 2026-08-09

### Added

- Publication readiness docs: CONTRIBUTING, SECURITY, known limitations, error catalog, spec/impl delta, release process.
- Dual-license files (`LICENSE-MIT`, `LICENSE-APACHE`).
- Web documentation site on branch `docs` (folder `docs/`; see #39).
- Struct field access on function parameters when the field uniquely identifies a struct (`E0043` when ambiguous) (#12).
- Enum unit/tuple variants + qualified `match` patterns (`Color.Red`, `Color.Custom(...)`); example `examples/enums` (#11, #34).
- `bool` literals in CIR/emit.
- Module stub pre-pass so mutual `use` across files resolves reliably (#13).
- Nested module paths (`math.vector`) emit a real Rust `mod` tree; example `examples/nested_math` (#35).
- Diagnostic UX: source snippets, caret, notes/help; unresolved-name module hints; match/`->` parse help (#22).
- CIR/Rust emit insta pins for `hello` and `math` (#25).
- Test harness: `test_` prefix for emitted tests; float assert epsilon (#17).

### Changed

- Shapes: parse remains, resolve rejects with **E0039** (unsupported) for defs, bounds, and named shape types (#21).
- Ownership probe emit hardened so `inventory` and related examples pass `crpc check` (#14).

## [1.1.0] — 2026-06

### Added

- Complex examples: `inventory`, `workshop`, `vec_ops`, `fallible_chain`, `async_spawn`, `unsafe_math`, `data_pipeline`, `abnormal_suite`, `design_patterns`, `float_demo`.
- Spec v0.2 abnormal-path tests (`spec_abnormal_v2`, lexer/parser abnormal suites).
- Float arithmetic in typeck; float literals and `**` (`.powf`) in CIR/emit.
- LSP analysis API (hover, inlay hints, call overlays, code lenses).

### Fixed

- `Vec::push` / `Vec::len` / `let mut` emit for `new()`.
- Ownership probe: `Ty::Var` → `_`; tighter borrow-check detection for `crpc check`.
- Clippy / rustfmt CI cleanliness (`-D warnings`).

## [1.0.0] — 2026-06

### Added

- Editor hardening: diagnostics formatting, fuzz smoke, conformance e2e matrix, criterion benches.
- Milestone 1.0 LSP analysis surface (spec §16.3).

## [0.9.0] — 2026-05

### Added

- Stdlib shims: `vec` (`new` / `push` / `len`), fs read stub, async/Tokio examples.
- Pattern matching, `spawn` / async-await, `extern "C"`, `unsafe` demos.
- `test` / `test_compile_fail` harness.

## [0.8.0] — 2026-05

### Added

- Tooling and package management: `crisp.toml`, sealed `crisp.lock`, dependency → Cargo.toml.

## [0.7.0] — 2026-04

### Added

- CIR generation and Rust emission pipeline (`crisp-cir`, `crisp-rust-emit`).

## [0.1.0] – [0.6.0]

Scaffold through ownership, regions, and error passes. See [ROADMAP.md](ROADMAP.md) for the full milestone history.

[Unreleased]: https://github.com/jose-compu/crisp/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/jose-compu/crisp/releases/tag/v1.2.0
[1.1.0]: https://github.com/jose-compu/crisp/releases/tag/v1.1.0
[1.0.0]: https://github.com/jose-compu/crisp/releases/tag/v1.0.0
[0.9.0]: https://github.com/jose-compu/crisp/releases/tag/v0.9.0
[0.8.0]: https://github.com/jose-compu/crisp/releases/tag/v0.8.0
[0.7.0]: https://github.com/jose-compu/crisp/releases/tag/v0.7.0
