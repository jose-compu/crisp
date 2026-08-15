# Error catalog (`E00xx`)

User-facing Crisp toolchain errors use codes in the `E00xx`–`E008x` range. Prefer the rendered message; this table is a navigation aid.

| Code | Area | Meaning / recovery |
|------|------|-------------------|
| E0034 | resolve | Duplicate definition in a module — rename or remove one item |
| E0035 | resolve | Unresolved name — check spelling/`use`; help names the defining module when known |
| E0036 | resolve | Name is private — export with `pub` or import from the defining module |
| E0037 | resolve | Symbol not exported from module |
| E0038 | resolve | Ambiguous import — qualify or narrow `use` |
| E0039 | resolve | Reserved (was shapes-unsupported); data shapes are supported — see [#61](https://github.com/jose-compu/crisp/issues/61) |
| E0044 | resolve | `use rust…` crate missing from `[dependencies]` — add with `rust = true` ([#41](https://github.com/jose-compu/crisp/issues/41)) |
| E0045 | resolve | Dependency used via `use rust…` must set `rust = true` |
| E0046 | resolve | `use rust…` requires `{ item, … }` import list |
| E0047 | resolve | Invalid `use rust` path — expect `use rust.<crate> { … }` |
| E0049 | resolve | Generic parameter shadows a type of the same name — rename the param or drop the binder ([#75](https://github.com/jose-compu/crisp/issues/75), [#78](https://github.com/jose-compu/crisp/issues/78)) |
| W0048 | resolve | Warning: bare `use <name>` binds a Crisp module that shares a name with a `rust = true` dep — use `use rust.<name> { … }` for the crate |
| E0040 | typeck | Unknown type — annotate params/fields or fix struct name |
| E0041 | typeck | Unknown name |
| E0042 | typeck | Resolve error wrapped into typeck |
| E0043 | typeck | Ambiguous field on unresolved param type — annotate the parameter (`x: Item`) |
| E0050 | ownership | Ownership contradicts explicit annotation — adjust `own`/`&`/`&mut` or usage |
| E0051 | ownership | Ownership analysis failed |
| E0052 | ownership | Resolve error during ownership |
| E0053 | ownership | Type error during ownership |
| E0054–E0056 | emit/resolve | Ownership/type/resolve errors during rustc fallback resolution |
| E0057 | emit | Could not produce borrow-checking probe Rust — often a **compiler bug** or incomplete probe; file an issue with `crisp check` output |
| E0058 | emit | rustc unavailable; fallback skipped |
| E0060–E0062 | regions | Region assignment failures / wrapped ownership or type errors |
| E0070–E0073 | errors / CIR / LSP | Fallibility set mismatches, or pipeline wrap of resolve/type/region/error |
| E0074 | resolve / CIR / LSP | Resolve error (also used as wrap code in several crates) |
| E0080 | seal | `crisp.lock` drift or missing/stale sealed API entry — regenerate lock or fix pub API |
| E0081 | tests | Runtime `cargo test` failed for injected tests |
| E0082 | tests | `test_compile_fail` unexpectedly passed analysis |
| E0083 | tests | `test_compile_fail` failed for the wrong reason |

## Notes

- Spec §17.4 examples may disagree on a few code numbers; see [SPEC_IMPL_DELTA.md](SPEC_IMPL_DELTA.md).
- A `rustc` error on **emitted** project code after a successful Crisp analysis is defined as a **`crisp` bug**, not a user error (spec §0.4).
