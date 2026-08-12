# Known limitations (Crisp v1.3)

This page documents behaviors that surprise users and are **not** always full language bugs. Tracked under publication epic [#1](https://github.com/jose-compu/crisp/issues/1). Formal deltas vs the draft spec: [SPEC_IMPL_DELTA.md](SPEC_IMPL_DELTA.md).

## Bootstrap status

- The compiler is a **Rust-hosted bootstrap** (`crpc`). Self-hosting (Crisp compiling Crisp) is ROADMAP Phase 2 / milestone **v2.0.0**, not shipped.
- Spec document remains **v0.2.0-draft**. Do not treat every spec paragraph as implemented.

## Modules

- Flat `src/*.crp` modules may import each other regardless of filename order (function stubs are registered before body checking; [#13](https://github.com/jose-compu/crisp/issues/13)). Nested layouts such as `src/math/vector.crp` emit a Rust `math` / `math/vector` module tree (see `examples/nested_math`; [#35](https://github.com/jose-compu/crisp/issues/35)).

## Types and expressions

- **Field access on function parameters:** if exactly one known struct has the field, the param type is inferred (`item.sku`). If several structs share the field name, annotate the param or you get `E0043` ([#12](https://github.com/jose-compu/crisp/issues/12)).
- **Enums / variant `match`:** unit and simple tuple variants work (`Color.Red`, `Color.Custom(r,g,b)` with `match color { … }`). See `examples/enums`. Remaining gaps: recursive enums beyond CIR `Box`, bare unqualified variant values, full exhaustiveness checking ([#11](https://github.com/jose-compu/crisp/issues/11) / [#34](https://github.com/jose-compu/crisp/issues/34)).
- `match name {` with a bare identifier scrutinee is supported; prefer `match (expr) { … }` for complex scrutinees.
- **`shape`** is reserved and parseable, but **not supported** yet: `shape` definitions and `shape` bounds fail resolve with `E0039` ([#21](https://github.com/jose-compu/crisp/issues/21)).
- **Inherent `impl Type` methods** work end-to-end (`Vec2.new`, `v.magnitude()`, nested modules); see `examples/vec2_methods`, `point_impl`, `feature_gallery` ([#20](https://github.com/jose-compu/crisp/issues/20)). **`trait` / `impl Trait for Type`** still lack a full path ([#50](https://github.com/jose-compu/crisp/issues/50)).
- Multi-module crates now emit main-module enums/structs alongside nested `mod` trees (regression fixed with `feature_gallery`).

## Tooling

- **`crpc check`** runs ownership probe emit + rustc (floats, `format!` interpolation, type defs). Remaining gaps may still yield non-borrow probe failures that are ignored rather than `E0057` ([#14](https://github.com/jose-compu/crisp/issues/14)).
- **Diagnostics:** resolve/typeck errors from `crpc check` render source snippets and hints when spans resolve (E0035 names the defining module when known) ([#22](https://github.com/jose-compu/crisp/issues/22)).
- **`test "name"`** becomes `fn test_<sanitized>` so it does not shadow crate items. Float `assert_eq` uses a small epsilon. Crisp string literals with unescaped quotes/`\` can still break harness emit ([#17](https://github.com/jose-compu/crisp/issues/17)).
- **`reveal` (spec §16):** `types` / `ownership` / `lifetimes` / `errors` / `rust` / `seal` are the deep overlays. Gaps vs the draft spec ([#19](https://github.com/jose-compu/crisp/issues/19)):
  - `expand` prints signatures + shallow `<inferred>` / `<body>` stubs, not a full annotated rewrite.
  - `diff` is a function-name summary, not a true Crisp↔Rust side-by-side.
  - `map` emits coarse CIR alloc/drop notes, not span-accurate annotations on emitted Rust.
  - `traits` only summarizes shape traits known to CIR; full `trait` / `impl Trait for` remains open under [#50](https://github.com/jose-compu/crisp/issues/50).
- **`crisp-lsp`:** analysis API only (`CrispAnalysis` — hover, inlay hints, call overlays, code lenses, emitted Rust). No stdio/`tower-lsp` host yet; see QUICKSTART §11 ([#18](https://github.com/jose-compu/crisp/issues/18)).

## Rust crate interop (spec §14.2)

- **`crisp.toml` → Cargo.toml** works: `[dependencies]` with `rust = true` are written into `target/rust/Cargo.toml`.
- **Primary import (TS-like):** `use serde_json { from_str }` when `serde_json` is a `rust = true` dependency — no `rust.` prefix required.
- **Compat alias:** `use rust.serde_json { … }` / `use rust::<crate> { … }` still force the Cargo crate.
- **Collision:** if a Crisp module and a Rust dep share a name, bare `use <name>` binds the **Crisp module** and emits **W0048**; use `use rust.<name> { … }` for the crate.
- Resolve codes: `E0044`–`E0047`, `W0048`. Bindings: `ResolvedRustImport` / `SymbolKind::RustFn`.
- **Calls:** known stubs (e.g. `serde_json::from_str` / `to_string`) typecheck and emit; Result APIs use `.expect(...)` in generated Rust until Crisp `?` absorbs Rust errors ([#41](https://github.com/jose-compu/crisp/issues/41)). See `examples/rust_import`.
- Epic plan: [#51](https://github.com/jose-compu/crisp/issues/51).

## Stdlib

- Shipped: `vec` shims (`new` / `push` / `len`), limited fs/async.
- Not shipped: Show/Eq/Ord shims ([#27](https://github.com/jose-compu/crisp/issues/27)), net/http ([#28](https://github.com/jose-compu/crisp/issues/28)), channels ([#38](https://github.com/jose-compu/crisp/issues/38)).
