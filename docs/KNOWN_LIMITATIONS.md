# Known limitations (Crisp v1.1)

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
- **`trait` / `impl` methods** parse but lack a full end-to-end example path ([#20](https://github.com/jose-compu/crisp/issues/20)).

## Tooling

- **`crpc check`** runs ownership probe emit + rustc (floats, `format!` interpolation, type defs). Remaining gaps may still yield non-borrow probe failures that are ignored rather than `E0057` ([#14](https://github.com/jose-compu/crisp/issues/14)).
- **Diagnostics:** resolve/typeck errors from `crpc check` render source snippets and hints when spans resolve (E0035 names the defining module when known) ([#22](https://github.com/jose-compu/crisp/issues/22)).
- **`test "name"`** becomes `fn test_<sanitized>` so it does not shadow crate items. Float `assert_eq` uses a small epsilon. Crisp string literals with unescaped quotes/`\` can still break harness emit ([#17](https://github.com/jose-compu/crisp/issues/17)).
- **`crisp-lsp`** exposes an analysis API; a full stdio LSP server is not shipped yet ([#18](https://github.com/jose-compu/crisp/issues/18)).

## Stdlib

- Shipped: `vec` shims (`new` / `push` / `len`), limited fs/async.
- Not shipped: Show/Eq/Ord shims ([#27](https://github.com/jose-compu/crisp/issues/27)), net/http ([#28](https://github.com/jose-compu/crisp/issues/28)), channels ([#38](https://github.com/jose-compu/crisp/issues/38)).
