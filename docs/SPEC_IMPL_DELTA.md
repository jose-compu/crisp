# Spec vs implementation delta (v0.2.0-draft ↔ crisp 1.7.1)

This document records known differences between [CrispLang-SPECS-0.2.0.md](spec/CrispLang-SPECS-0.2.0.md) and the current bootstrap compiler. It is not exhaustive; see also [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) and abnormal-path tests.

## Status claims

| Spec / docs | Reality |
|-------------|---------|
| Spec status “Abstract Design Specification” / draft | Still draft; not a frozen language standard |
| ROADMAP “spec-complete bootstrap” at 1.0 | Overstates remaining trait/shape/enum/stdlib gaps; prefer “bootstrap shipped with known gaps” |

## Error codes

| Topic | Spec | Implementation |
|-------|------|----------------|
| Ownership contradiction | §17.4 examples may show `E0042` | Ownership pass uses **`E0050`** (`crisp-ownership`) |
| Unknown name | May cite resolve codes | Resolve **`E0035`**; typeck often wraps as **`E0041`** / **`E0042`** |

## Analysis surface of `crisp check` / `test_compile_fail`

- `crisp check` runs resolve, typeck, ownership (+ rustc probe fallbacks), regions, error pass, sealed verify.
- `test_compile_fail` harness typically runs **typecheck-oriented** failure (not always full ownership/error passes). See `crates/crisp-rust-emit/src/test_harness.rs`.

## Language features (high level)

| Feature | Spec | Impl (through 1.7.1) |
|---------|------|--------------|
| Structs, defaults, sealed lock | §3.3, §12.5 | Working; examples |
| Float + `**` | §3, operators | Working (recent); examples `math` / `float_demo` |
| Enums + variant match | §3.3.2, §6.2, §10 | Working for unit/tuple variants + qualified patterns (`examples/enums`); exhaustiveness / recursive polish TBD |
| Inherent `impl Type` methods | §5.4 | Working (`examples/vec2_methods`, `point_impl`); associated `new` + `self` methods |
| Traits / `impl Trait for` | §3.6 | Method traits + literal/simple defaults (`examples/show_trait`, `trait_defaults`); bounds / dyn still limited — [#59](https://github.com/jose-compu/crisp/issues/59) |
| Rust crate imports | §14.2 | Parse + resolve + call emit: bare `use crate { … }` when `rust = true`; `use rust.` alias; W0048 on name shadow; stubs for `serde_json` / `ureq`; known Result APIs → `CrispError::Thrown` + `?` (#55) |
| Closures / lambdas | §5.3 | Function values + implicit sugar (holes, trailing last-arg, `.field`, `.m()`) — `examples/closures` (#72, #87–#89). No operator sections; no `dyn Fn` |
| Shapes | §3.5 | Data shapes + parametric `shape Name<T>` (`examples/shapes`, `examples/generics`, `examples/shapes_generic`); method/anonymous shapes still limited ([#61](https://github.com/jose-compu/crisp/issues/61), [#70](https://github.com/jose-compu/crisp/issues/70)) |
| User generics | §3 / §5 | **Prefer implicit binders** (`examples/generics_implicit`) — [#75](https://github.com/jose-compu/crisp/issues/75); explicit pins (`examples/generics`) — [#71](https://github.com/jose-compu/crisp/issues/71); inferred `impl Trait for Type` args + `+` bounds — [#77](https://github.com/jose-compu/crisp/issues/77); unannotated `id(x)=x` generalizes; internal single-use monomorphizes; `pub` schemes seal in `crisp.lock` (`examples/generics_pub`) — [#76](https://github.com/jose-compu/crisp/issues/76); value restriction + reveal `T: Clone` — [#78](https://github.com/jose-compu/crisp/issues/78). Remaining: anonymous `{ value: T }` params, `where` / HRTB/GATs |
| Channels | §11.4 | Not implemented |
| Std Show/Eq/Ord | §15.4 | Prelude shims + Display/PartialEq/Ord bridges (`examples/std_traits`) — [#27](https://github.com/jose-compu/crisp/issues/27). Add/Sub/Mul/Div inferred from operators on `T`; typeck E0084 on bad instantiations; unique method → nullary trait bound (`examples/shapes_generic`, `examples/show_trait`, `examples/shapes_user`) — [#84](https://github.com/jose-compu/crisp/issues/84) |
| Std net/http | §15.2 | `std.net.parse_ip` + thin `ureq` GET (`examples/net_http`); full `std.http` server API deferred — [#28](https://github.com/jose-compu/crisp/issues/28) |
| Nested modules | §12 | Resolve + emit nest `a.b` as `mod a` / `a/b.rs` (`examples/nested_math`) |
| Loops | §6.3 | `while` / `for` / `loop` + `break`/`continue` / value `break` (`examples/loops`); `for` MVP over `vec` only; no labels / `enumerate` yet |

## Tooling: `reveal` (§16) and LSP (§16.3)

Beginner overview: [QUICKSTART §10](../QUICKSTART.md#10-inspect-what-the-compiler-inferred-reveal) (`crisp` = build/run; `reveal` = inspect inference).

| Spec command | Implementation |
|--------------|----------------|
| `reveal types` | Implemented (`reveal_types`) |
| `reveal ownership` | Implemented (+ rustc fallbacks) |
| `reveal lifetimes` | Implemented |
| `reveal errors` | Implemented |
| `reveal traits` | User traits + trait impls from CIR; shape traits when present |
| `reveal rust` | Implemented (crate entry via emit pipeline) |
| `reveal seal` | Implemented |
| `reveal expand` | Partial — signature + shallow body stubs |
| `reveal diff` | Partial — fn-name summary, not side-by-side |
| `reveal map` | Partial — coarse CIR notes |
| §16.3 LSP host | Stdio `crisp-lsp` binary (hover, inlays, diagnostics) + `CrispAnalysis` library (#56) |

CLI: `crates/crpc/src/reveal.rs` (binary from `-p crisp-lang`). Docs: QUICKSTART §10–§11, KNOWN_LIMITATIONS.

## Ownership probe (§7.6)

Probe emit covers floats, `format!` interpolation, and type defs for common cases. Non-borrow rustc failures on the probe are still not treated as §7.6 disagreements (`resolve.rs`). Residual gaps: [#14](https://github.com/jose-compu/crisp/issues/14).

## Tests that document deltas

- `crates/crisp-rust-emit/tests/spec_abnormal_v2.rs`
- `examples/abnormal_suite`
- Lexer/parser abnormal tests under `crates/crisp-lexer`, `crates/crisp-parser`
