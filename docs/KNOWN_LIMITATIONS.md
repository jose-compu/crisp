# Known limitations (Crisp v1.7.1)

This page documents behaviors that surprise users and are **not** always full language bugs. Formal deltas vs the draft spec: [SPEC_IMPL_DELTA.md](SPEC_IMPL_DELTA.md). Open follow-ups: crates.io ([#66](https://github.com/jose-compu/crisp/issues/66)), visibility ([#58](https://github.com/jose-compu/crisp/issues/58)), trait bounds/`dyn` ([#59](https://github.com/jose-compu/crisp/issues/59)).

## Bootstrap status

- The compiler is a **Rust-hosted bootstrap** (`crisp`). Self-hosting (Crisp compiling Crisp) is ROADMAP Phase 2 / milestone **v2.0.0**, not shipped.
- Spec document remains **v0.2.0-draft**. Do not treat every spec paragraph as implemented.
- **Install:** `cargo install crisp-lang --locked` (also installs `reveal`). LSP: `cargo install crisp-lsp --locked`. From a clone: `cargo install --path crates/crpc --locked`. See [CRATES_IO.md](CRATES_IO.md) ([#66](https://github.com/jose-compu/crisp/issues/66)).

## Modules

- Flat `src/*.crp` modules may import each other regardless of filename order (function stubs are registered before body checking; [#13](https://github.com/jose-compu/crisp/issues/13)). Nested layouts such as `src/math/vector.crp` emit a Rust `math` / `math/vector` module tree with `crate::` paths for intra-crate calls (see `examples/nested_math`; [#35](https://github.com/jose-compu/crisp/issues/35), [#93](https://github.com/jose-compu/crisp/issues/93)).

## Types and expressions

- **Field access on function parameters:** if exactly one known struct has the field, the param type is inferred (`item.sku`). If several structs share the field name, annotate the param or you get `E0043` ([#12](https://github.com/jose-compu/crisp/issues/12)).
- **Enums / variant `match`:** unit and simple tuple variants work (`Color.Red`, `Color.Custom(r,g,b)` with `match color { … }`). See `examples/enums`. Remaining gaps: recursive enums beyond CIR `Box`, bare unqualified variant values, full exhaustiveness checking ([#11](https://github.com/jose-compu/crisp/issues/11) / [#34](https://github.com/jose-compu/crisp/issues/34)).
- `match name {` with a bare identifier scrutinee is supported; prefer `match (expr) { … }` for complex scrutinees.
- **`shape`** data shapes work end-to-end for field accessors (`examples/shapes`): generated trait + structural call sites (§3.5). Parametric shapes (`shape Boxy<T>`) work (`examples/generics`, [#70](https://github.com/jose-compu/crisp/issues/70)). Remaining gaps: method shapes, anonymous `{ x: float }` params, full bound/`dyn`-style use ([#61](https://github.com/jose-compu/crisp/issues/61)).
- **Function values** (spec §5.2–§5.3): one callable kind — named items and `|x| …` are the same. Holes (`_ * 2`), trailing last-arg (`run { |x| … }`), field sections (`.name`), method sections (`.magnitude()`, `.scale(2.0)` baked args) — `examples/closures` (#72, #87–#89). Remaining: operator sections (`+ 1`), `dyn Fn`.
- **Generics (prefer implicit):** write `id(x: T)`, `type Pair = { left: A, right: B }`, `shape Boxy = { value: T }` — unbound type names are parameters (`examples/generics_implicit`, [#75](https://github.com/jose-compu/crisp/issues/75)). `<>` is a pin and is used for applications (`Pair<int, str>`, `Boxy<int>`). Explicit pins also work (`examples/generics`, [#71](https://github.com/jose-compu/crisp/issues/71)). `impl Wrapper for IntBox` infers `Wrapper<int>`; `item: HasName + HasId` emits extra Rust bounds ([#77](https://github.com/jose-compu/crisp/issues/77)). Typeck checks the first (inner) constraint; further `+` bounds are enforced by `rustc`. In-scope types stay types; `<T>` shadowing a type is E0049 ([#78](https://github.com/jose-compu/crisp/issues/78)). Unannotated `id(x) = x` generalizes when free vars remain; crate-internal single-instantiation items monomorphize; `pub` schemes freeze in `crisp.lock` (E0080 on drift) — `examples/generics_pub` ([#76](https://github.com/jose-compu/crisp/issues/76)). Locals and `mut` bindings are not generalized; `reveal types` shows `T: Clone` (emit/lock contract). Remaining: anonymous `{ value: T }` params, `where` / HRTB/GATs. Nested applications that end in `>>` (`Pair<T, Pair<A, B>>`) are lexed as a shift token — write a space (`> >`) or avoid nesting for now.
- **Inherent `impl Type` methods** work end-to-end (`Vec2.new`, `v.magnitude()`, nested modules); see `examples/vec2_methods`, `point_impl`, `feature_gallery` ([#20](https://github.com/jose-compu/crisp/issues/20)).
- **`trait` / `impl Trait for Type`** work for method traits (`examples/show_trait`). Literal/simple **default method bodies** emit (`examples/trait_defaults`). Unique method use on generic `T` infers a nullary bound (`label(x: T) = x.show()` → `T: Show`; `examples/shapes_user` `T: Measure`) and **E0084** rejects instantiations without an impl ([#84](https://github.com/jose-compu/crisp/issues/84)). Remaining gaps: complex default bodies, generic trait bounds, `dyn Trait` ([#59](https://github.com/jose-compu/crisp/issues/59)).
- **Loops (`while` / `for` / `loop`):** work end-to-end (`examples/loops`), including `break` / `continue`, value-producing `break <expr>` on `loop`, and assignment in `if then` / `else` inside `while` (float bisection, [#96](https://github.com/jose-compu/crisp/issues/96)). `for` currently iterates Crisp `vec` only (emits `.iter().copied()`); no `enumerate`, labels, or general `IntoIterator` yet. `if`/`while`/`for` heads disallow bare struct literals so `{` starts the body (parenthesize struct lits in conditions when needed).
- **Std Show / Eq / Ord** are prelude shims (`show` / `equal` / `compare`) with Rust `Display` / `PartialEq`+`Eq` / `Ord` bridges (`examples/std_traits`) ([#27](https://github.com/jose-compu/crisp/issues/27)). **Add / Sub / Mul / Div** are inferred from operators on generic `T` (`T: Clone + Add + …` → `std::ops`); typeck **E0084** rejects instantiations that lack the bound ([#84](https://github.com/jose-compu/crisp/issues/84)). Unique method use on `T` infers a nullary user trait (`T: Show`). `impl Add for Type` / generic trait bounds / `dyn Trait` remain limited ([#59](https://github.com/jose-compu/crisp/issues/59)).
- Multi-module crates emit main-module enums/structs alongside nested `mod` trees.

## Tooling

- **`crisp check`** runs ownership probe emit + rustc (floats, `format!` interpolation, type defs). Remaining gaps may still yield non-borrow probe failures that are ignored rather than `E0057` ([#14](https://github.com/jose-compu/crisp/issues/14)).
- **Diagnostics:** resolve/typeck errors from `crisp check` render source snippets and hints when spans resolve (E0035 names the defining module when known; interpolation `{name}` points at the string, not the first `use`) ([#22](https://github.com/jose-compu/crisp/issues/22), [#95](https://github.com/jose-compu/crisp/issues/95)).
- **`test "name"`** becomes `fn test_<sanitized>` so it does not shadow crate items. Float `assert_eq` uses a small epsilon. Crisp string literals with unescaped quotes/`\` can still break harness emit ([#17](https://github.com/jose-compu/crisp/issues/17)).
- **`reveal` (spec §16):** beginner walkthrough in [QUICKSTART §10](../QUICKSTART.md#10-inspect-what-the-compiler-inferred-reveal). Deep overlays today: `types` / `ownership` / `lifetimes` / `errors` / `rust` / `seal` / user `traits`. Gaps ([#19](https://github.com/jose-compu/crisp/issues/19)): `expand` / `diff` / `map` remain shallow.
- **`crisp-lsp`:** stdio LSP binary (`cargo install --path crates/crisp-lsp`) — hover, inlay hints, crate diagnostics on open/save ([#56](https://github.com/jose-compu/crisp/issues/56)). Library API: `CrispAnalysis`.
- **Editor packaging:** [`editors/vscode-crisp`](../editors/vscode-crisp) + `./scripts/package-vsix.sh` ([#57](https://github.com/jose-compu/crisp/issues/57)). Marketplace listing still optional.

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
