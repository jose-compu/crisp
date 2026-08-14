# Changelog

All notable changes to the Crisp toolchain (`crisp`, `reveal`, and workspace crates) are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

Language edition (`crisp.toml` `edition = "2026"`) tracks the abstract language design; compiler versions below are independent.

## [Unreleased]

## [1.5.2] — 2026-08-14

### Changed

- CLI binary **`crpc` → `crisp`**; crates.io package **`crisp-crpc` → `crisp-lang`**. Docs, scripts, and tests updated (#68).
- Workspace version **1.5.1 → 1.5.2** so crates.io can republish after the rename (1.5.1 already occupied the index for early crates) (#66).

### Added

- crates.io publish prep: workspace `homepage` / `repository` inheritance, [CRATES_IO.md](docs/CRATES_IO.md), `scripts/publish-crates.sh` (#66).

## [1.5.1] — 2026-08-13

### Added

- Loop constructs end-to-end (spec §6.3): `while`, `for … in`, `loop`, `break` / `break <expr>`, `continue` — typeck, CIR, Rust emit, probe emit (`examples/loops`).
- Parser: disable struct-literal greed in `if` / `while` / `for` heads so `while i < n { … }` parses (Rust-style restriction).
- `mut:=` bindings now emit `let mut` in CIR/Rust (needed for loop counters).

### Notes

- `for` MVP iterates Crisp `vec` (`Vec<i64>`) via `.iter().copied()`. `enumerate`, labeled breaks, and non-vec iterators remain future work.
- Still open: crates.io (#66), repo visibility (#58); trait bounds/`dyn` remain partial (#59).

## [1.5.0] — 2026-08-12

### Added

- Absorb known Rust `Result` APIs into Crisp ambient errors: `.map_err(|e| CrispError::Thrown(...))?` for `serde_json` / `ureq` imports (#55).
- `#[derive(Debug)]` on generated `CrispError` so fallible `main` can return `Result<(), CrispError>`.

### Changed

- Workspace version **1.5.0** — first public release track.
- Docs / site framing for public launch; use-case pages with longer example snippets and GitHub links.
- [KNOWN_LIMITATIONS](docs/KNOWN_LIMITATIONS.md) updated for Result absorption.

### Notes

- Remaining v1.5.0 board items after start PR: stdio LSP (#56), editor package (#57), trait polish (#59), shapes (#61), crates.io (#60). Visibility flip: (#58).

## [1.4.1] — 2026-08-12

### Changed

- Public-preview hygiene: CI badge, README motto + Result sharp-edge callout, homepage/topics for GitHub Pages.
- [KNOWN_LIMITATIONS](docs/KNOWN_LIMITATIONS.md) brought current for v1.4 (Show/Eq/Ord, net/http, interop `.expect`).
- [RELEASE.md](docs/RELEASE.md) / [CONTRIBUTING.md](CONTRIBUTING.md) public checklist updated; deferred work under milestone [v1.5.0](https://github.com/jose-compu/crisp/milestone/6).
- Workspace version bump to **1.4.1**.

### Notes

- Repository visibility flip remains a maintainer action ([#58](https://github.com/jose-compu/crisp/issues/58) · milestone [v1.4.1](https://github.com/jose-compu/crisp/milestone/7)).

## [1.4.0] — 2026-08-12

### Added

- TypeScript-style Rust crate imports: bare `use serde_json { … }` when the crate is a `rust = true` dependency; `use rust.<crate>` / `use rust::<crate>` kept as aliases (`E0044`–`E0047`, `ResolvedRustImport`) (#41).
- **W0048** when a Crisp module and a Rust dep share a name: bare `use` binds the module; `use rust.<name>` selects the crate.
- Examples: `examples/rust_import`, `examples/rust_shadow`.
- Typeck/emit stubs for imported Rust fns (`serde_json`, `ureq`); Result APIs emit `.expect(...)` until Crisp `?` absorbs Rust errors.
- `trait` / `impl Trait for Type` through typeck → CIR → emit (`examples/show_trait`) (#50).
- Prelude **Show / Eq / Ord** shims (§15.4): emit Crisp traits + Rust `Display` / `PartialEq`+`Eq` / `Ord` bridges; methods `show` / `equal` / `compare` (`examples/std_traits`) (#27).
- **std.net.parse_ip** shim + **ureq** HTTP GET via `rust = true` (`examples/net_http`) (#28).
- VS Code / Cursor TextMate extension for `.crp` (`editors/vscode-crisp`).
- Beginner-oriented `reveal` documentation (QUICKSTART §10).
- CIR/emit: `&&` / `||` / `!=` / `<=` / `>=` binary ops (no longer mis-lowered to `+`).

### Notes

- Rust `Result` from imported crates still uses `.expect(...)` (follow-up for `?` absorption).
- Full Crisp `std.http` server API (§20) remains deferred; #28 ships thin re-exports.

## [1.3.0] — 2026-08-12

### Added

- CI: Ubuntu + macOS matrix; dedicated MSRV **1.85** job; `rust-toolchain.toml` (stable + rustfmt/clippy) (#23).
- `reveal` CLI: richer `--help` / subcommand docs and clearer path errors (#19).
- QUICKSTART: full `reveal` command table (§16 fidelity notes) and `crisp-lsp` analysis-API usage (#18, #19).
- Inherent `impl Type` methods end-to-end: typeck, ownership/errors/regions, CIR `AssocCall`/`MethodCall`, emit (`&self`) (#20).
- Examples: `vec2_methods` (nested + methods), `point_impl`, `feature_gallery` (nested + enums + methods).

### Changed

- Document `crisp-lsp` as analysis-API-only until a stdio host ships (#18).
- CONTRIBUTING: install `reveal` via `-p crisp-lang` only (library `crisp-reveal` is not installable as a binary).
- Multi-module emit writes main-module structs/enums/impls (not only functions).
- Test harness emits instance/associated method calls (no more `unknown()`).

### Notes

- `trait` / `impl Trait for Type` deferred to [#50](https://github.com/jose-compu/crisp/issues/50) (v1.4.0).

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
- Ownership probe emit hardened so `inventory` and related examples pass `crisp check` (#14).

## [1.1.0] — 2026-06

### Added

- Complex examples: `inventory`, `workshop`, `vec_ops`, `fallible_chain`, `async_spawn`, `unsafe_math`, `data_pipeline`, `abnormal_suite`, `design_patterns`, `float_demo`.
- Spec v0.2 abnormal-path tests (`spec_abnormal_v2`, lexer/parser abnormal suites).
- Float arithmetic in typeck; float literals and `**` (`.powf`) in CIR/emit.
- LSP analysis API (hover, inlay hints, call overlays, code lenses).

### Fixed

- `Vec::push` / `Vec::len` / `let mut` emit for `new()`.
- Ownership probe: `Ty::Var` → `_`; tighter borrow-check detection for `crisp check`.
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

[Unreleased]: https://github.com/jose-compu/crisp/compare/v1.5.0...HEAD
[1.5.0]: https://github.com/jose-compu/crisp/compare/v1.4.1...v1.5.0
[1.4.1]: https://github.com/jose-compu/crisp/compare/v1.4.0...v1.4.1
[1.4.0]: https://github.com/jose-compu/crisp/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/jose-compu/crisp/releases/tag/v1.3.0
[1.2.0]: https://github.com/jose-compu/crisp/releases/tag/v1.2.0
[1.1.0]: https://github.com/jose-compu/crisp/releases/tag/v1.1.0
[1.0.0]: https://github.com/jose-compu/crisp/releases/tag/v1.0.0
[0.9.0]: https://github.com/jose-compu/crisp/releases/tag/v0.9.0
[0.8.0]: https://github.com/jose-compu/crisp/releases/tag/v0.8.0
[0.7.0]: https://github.com/jose-compu/crisp/releases/tag/v0.7.0
