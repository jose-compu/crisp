# Spec vs implementation delta (v0.2.0-draft ↔ crpc 1.1.x)

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

## Analysis surface of `crpc check` / `test_compile_fail`

- `crpc check` runs resolve, typeck, ownership (+ rustc probe fallbacks), regions, error pass, sealed verify.
- `test_compile_fail` harness typically runs **typecheck-oriented** failure (not always full ownership/error passes). See `crates/crisp-rust-emit/src/test_harness.rs`.

## Language features (high level)

| Feature | Spec | Impl (1.1.x) |
|---------|------|--------------|
| Structs, defaults, sealed lock | §3.3, §12.5 | Working; examples |
| Float + `**` | §3, operators | Working (recent); examples `math` / `float_demo` |
| Enums + variant match | §3.3.2, §6.2, §10 | Working for unit/tuple variants + qualified patterns (`examples/enums`); exhaustiveness / recursive polish TBD |
| Traits / inherent `impl` | §3.6, §5.4 | Parse/AST; typeck/CIR incomplete for bodies |
| Shapes | §3.5 | Keyword + partial CIR synthesis; typeck incomplete |
| Channels | §11.4 | Not implemented |
| Std Show/Eq/Ord, net/http | §15 | Not implemented |
| Nested modules | §12 | Resolve walks dirs; emit nesting incomplete |

## Ownership probe (§7.6)

Probe emit is intentionally **partial**. Non-borrow rustc failures on the probe are not treated as §7.6 disagreements (`resolve.rs`). Remaining probe gaps: [#14](https://github.com/jose-compu/crisp/issues/14).

## Tests that document deltas

- `crates/crisp-rust-emit/tests/spec_abnormal_v2.rs`
- `examples/abnormal_suite`
- Lexer/parser abnormal tests under `crates/crisp-lexer`, `crates/crisp-parser`
