# Known limitations (Crisp v1.1)

This page documents behaviors that surprise users and are **not** always full language bugs. Tracked under publication epic [#1](https://github.com/jose-compu/crisp/issues/1). Formal deltas vs the draft spec: [SPEC_IMPL_DELTA.md](SPEC_IMPL_DELTA.md).

## Bootstrap status

- The compiler is a **Rust-hosted bootstrap** (`crpc`). Self-hosting (Crisp compiling Crisp) is ROADMAP Phase 2 / milestone **v2.0.0**, not shipped.
- Spec document remains **v0.2.0-draft**. Do not treat every spec paragraph as implemented.

## Modules

- In a flat `src/` directory, modules imported by `main` must sort **alphabetically before** `main.crp` or names may not resolve. Prefer names like `hub.crp` over `mediator.crp` when `main` imports them. Fix tracked in [#13](https://github.com/jose-compu/crisp/issues/13).
- Nested `math/vector.crp` layouts are specified (§12) but emit of nested Rust `mod` trees is incomplete ([#35](https://github.com/jose-compu/crisp/issues/35)).

## Types and expressions

- **Field access on function parameters:** if exactly one known struct has the field, the param type is inferred (`item.sku`). If several structs share the field name, annotate the param or you get `E0043` ([#12](https://github.com/jose-compu/crisp/issues/12)).
- **Enums / variant `match`** are partially implemented; many examples avoid them. Prefer int/string demos until [#11](https://github.com/jose-compu/crisp/issues/11) / [#34](https://github.com/jose-compu/crisp/issues/34) land.
- `match x {` without parentheses can be parsed as a **struct literal**; use `match (x) { … }` when in doubt.
- **`shape`** is a reserved keyword (cannot use as a field name). Shape types are not fully implemented ([#21](https://github.com/jose-compu/crisp/issues/21)).
- **`trait` / `impl` methods** parse but lack a full end-to-end example path ([#20](https://github.com/jose-compu/crisp/issues/20)).

## Tooling

- **`crpc check`** runs ownership probe emit + rustc. Probe emission is still partial (interpolation, some exprs); false `E0057` is possible ([#14](https://github.com/jose-compu/crisp/issues/14)).
- **`test "name"`** sanitizes to a Rust `fn` that can **shadow** a same-named `pub fn` in tests (e.g. avoid `test "proxy demo"` if `proxy_demo` exists). See [#17](https://github.com/jose-compu/crisp/issues/17).
- **`crisp-lsp`** exposes an analysis API; a full stdio LSP server is not shipped yet ([#18](https://github.com/jose-compu/crisp/issues/18)).

## Stdlib

- Shipped: `vec` shims (`new` / `push` / `len`), limited fs/async.
- Not shipped: Show/Eq/Ord shims ([#27](https://github.com/jose-compu/crisp/issues/27)), net/http ([#28](https://github.com/jose-compu/crisp/issues/28)), channels ([#38](https://github.com/jose-compu/crisp/issues/38)).
