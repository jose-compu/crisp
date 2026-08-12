# Known limitations (Crisp v1.5)

This page documents behaviors that surprise users and are **not** always full language bugs. Formal deltas vs the draft spec: [SPEC_IMPL_DELTA.md](SPEC_IMPL_DELTA.md). Publication backlog: [milestone v1.5.0](https://github.com/jose-compu/crisp/milestone/6).

## Bootstrap status

- The compiler is a **Rust-hosted bootstrap** (`crpc`). Self-hosting (Crisp compiling Crisp) is ROADMAP Phase 2 / milestone **v2.0.0**, not shipped.
- Spec document remains **v0.2.0-draft**. Do not treat every spec paragraph as implemented.
- **Public preview:** install from source (`cargo install --path crates/crpc --locked`). crates.io publish is deferred ([#60](https://github.com/jose-compu/crisp/issues/60)).

## Modules

- Flat `src/*.crp` modules may import each other regardless of filename order (function stubs are registered before body checking; [#13](https://github.com/jose-compu/crisp/issues/13)). Nested layouts such as `src/math/vector.crp` emit a Rust `math` / `math/vector` module tree (see `examples/nested_math`; [#35](https://github.com/jose-compu/crisp/issues/35)).

## Types and expressions

- **Field access on function parameters:** if exactly one known struct has the field, the param type is inferred (`item.sku`). If several structs share the field name, annotate the param or you get `E0043` ([#12](https://github.com/jose-compu/crisp/issues/12)).
- **Enums / variant `match`:** unit and simple tuple variants work (`Color.Red`, `Color.Custom(r,g,b)` with `match color { … }`). See `examples/enums`. Remaining gaps: recursive enums beyond CIR `Box`, bare unqualified variant values, full exhaustiveness checking ([#11](https://github.com/jose-compu/crisp/issues/11) / [#34](https://github.com/jose-compu/crisp/issues/34)).
- `match name {` with a bare identifier scrutinee is supported; prefer `match (expr) { … }` for complex scrutinees.
- **`shape`** is reserved and parseable, but **not supported** yet: `shape` definitions and `shape` bounds fail resolve with `E0039` ([#61](https://github.com/jose-compu/crisp/issues/61) → v1.5.0).
- **Inherent `impl Type` methods** work end-to-end (`Vec2.new`, `v.magnitude()`, nested modules); see `examples/vec2_methods`, `point_impl`, `feature_gallery` ([#20](https://github.com/jose-compu/crisp/issues/20)).
- **`trait` / `impl Trait for Type`** work for simple method traits (`examples/show_trait`). Remaining gaps: default bodies, bound enforcement, `dyn Trait` ([#59](https://github.com/jose-compu/crisp/issues/59)).
- **Std Show / Eq / Ord** are prelude shims (`show` / `equal` / `compare`) with Rust `Display` / `PartialEq`+`Eq` / `Ord` bridges (`examples/std_traits`) ([#27](https://github.com/jose-compu/crisp/issues/27)).
- Multi-module crates emit main-module enums/structs alongside nested `mod` trees.

## Tooling

- **`crpc check`** runs ownership probe emit + rustc (floats, `format!` interpolation, type defs). Remaining gaps may still yield non-borrow probe failures that are ignored rather than `E0057` ([#14](https://github.com/jose-compu/crisp/issues/14)).
- **Diagnostics:** resolve/typeck errors from `crpc check` render source snippets and hints when spans resolve (E0035 names the defining module when known) ([#22](https://github.com/jose-compu/crisp/issues/22)).
- **`test "name"`** becomes `fn test_<sanitized>` so it does not shadow crate items. Float `assert_eq` uses a small epsilon. Crisp string literals with unescaped quotes/`\` can still break harness emit ([#17](https://github.com/jose-compu/crisp/issues/17)).
- **`reveal` (spec §16):** beginner walkthrough in [QUICKSTART §10](../QUICKSTART.md#10-inspect-what-the-compiler-inferred-reveal). Deep overlays today: `types` / `ownership` / `lifetimes` / `errors` / `rust` / `seal` / user `traits`. Gaps ([#19](https://github.com/jose-compu/crisp/issues/19)): `expand` / `diff` / `map` remain shallow.
- **`crisp-lsp`:** analysis API only (`CrispAnalysis`). No stdio LSP host yet ([#18](https://github.com/jose-compu/crisp/issues/18) / [#56](https://github.com/jose-compu/crisp/issues/56)).
- **Editor highlighting:** in-tree TextMate grammar at [`editors/vscode-crisp`](../editors/vscode-crisp) (symlink / F5). Marketplace / VSIX packaging tracked in [#57](https://github.com/jose-compu/crisp/issues/57).

## Rust crate interop (spec §14.2)

- **`crisp.toml` → Cargo.toml** works: `[dependencies]` with `rust = true` are written into `target/rust/Cargo.toml`.
- **Primary import (TS-like):** `use serde_json { from_str }` when `serde_json` is a `rust = true` dependency — no `rust.` prefix required.
- **Compat alias:** `use rust.serde_json { … }` / `use rust::<crate> { … }` still force the Cargo crate.
- **Collision:** if a Crisp module and a Rust dep share a name, bare `use <name>` binds the **Crisp module** and emits **W0048**; use `use rust.<name> { … }` for the crate.
- Resolve codes: `E0044`–`E0047`, `W0048`. Bindings: `ResolvedRustImport` / `SymbolKind::RustFn`.
- **Result absorption (#55):** known stubs (`serde_json::from_str` / `to_string`, `ureq::get`) lower to `.map_err(|e| CrispError::Thrown(...))?` and mark the enclosing function fallible. Absorb with `catch`, or let `main` return `Result<(), CrispError>`. Unknown Rust APIs are still opaque stubs without automatic `Result` mapping.

## Stdlib

- **Shipped:** `vec` (`new` / `push` / `len`), limited fs/async, prelude Show/Eq/Ord ([#27](https://github.com/jose-compu/crisp/issues/27)), `std.net.parse_ip`, thin HTTP via `ureq` ([#28](https://github.com/jose-compu/crisp/issues/28)).
- **Not shipped:** full Crisp `std.http` server API (§20), channels ([#38](https://github.com/jose-compu/crisp/issues/38) → v2.0.0), most of the §15 trait catalog beyond Show/Eq/Ord.
