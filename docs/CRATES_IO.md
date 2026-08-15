# crates.io publish (v1.6.0)

First registry publication for the Crisp toolchain. Tracking issue: [#66](https://github.com/jose-compu/crisp/issues/66).

## Package naming

| crates.io package | Binaries / role |
|-------------------|-----------------|
| `crisp-ast` … `crisp-reveal` | Library pipeline |
| **`crisp-lang`** | Bins **`crisp`** + **`reveal`**. Package names `crisp` / `crpc` / `crisp-cli` are taken on crates.io. |
| `crisp-lsp` | Stdio LSP binary |

Install:

```bash
cargo install crisp-lang --locked   # installs crisp + reveal
cargo install crisp-lsp --locked
crisp --version
```

Do **not** publish example crates under `examples/`.

**Publish cycle note:** `crisp-rust-emit` must not depend on `crisp-lsp` (even as a
dev-dependency with a version). `crisp-lsp` depends on `crisp-rust-emit`, so a
versioned `crisp-lsp` dep would block packaging until `crisp-lsp` already existed
on crates.io. LSP analysis tests live under `crates/crisp-lsp/tests/`.

## Publish order

Bottom-up so path deps resolve to registry versions (must match `scripts/publish-crates.sh`):

1. `crisp-ast`
2. `crisp-lexer`
3. `crisp-manifest`
4. `crisp-diagnostics`
5. `crisp-parser`
6. `crisp-resolve`
7. `crisp-typeck`
8. `crisp-ownership`
9. `crisp-errors`
10. `crisp-regions`
11. `crisp-cir`
12. `crisp-rust-emit`
13. `crisp-reveal`
14. `crisp-lang` (bins `crisp`, `reveal`)
15. `crisp-lsp`

The script validates this list against `cargo metadata` so a new workspace member cannot be forgotten. Dev-only edges (e.g. `crisp-rust-emit` → `crisp-lsp`) do not affect order.

Helper (dry-run by default):

```bash
./scripts/publish-crates.sh          # dry-run; skips crate@version already on crates.io
./scripts/publish-crates.sh --execute  # real publish; waits/retries on 429
```

Already-published versions are skipped. After each upload the script polls crates.io until `name@version` is visible, then pauses briefly (default 15s / 5s) so the next crate can resolve the new registry dep. crates.io still enforces **new crate** limits (burst of 5, then ~1 / 10 min) and **new version** limits (burst of 30, then ~1 / min); on 429 the script parses `try again after … GMT`, waits that long, and retries.

## Preconditions

1. `[workspace.package] version` is **1.6.0** (or the tag you are shipping). Internal `[workspace.dependencies]` entries must include matching `version = "…"` alongside `path` so publish can rewrite to crates.io.
2. `cargo fmt` / `clippy -D warnings` / `cargo test --workspace` green.
3. Each package has `description`, `license`, `repository` (inherited from workspace).
4. Name availability: `crisp` / `crpc` / `crisp-cli` are **taken**; publish CLI as **`crisp-lang`** (bins `crisp` / `reveal`). Library `crisp-*` names checked available (2026-08-14). See [#68](https://github.com/jose-compu/crisp/issues/68).
5. `cargo login` with a token that can publish under the intended owners.

## After publish

- Tag `v1.6.0` and GitHub Release notes.
- README / QUICKSTART / docs site: prefer

```bash
cargo install crisp-lang --locked
cargo install crisp-lsp --locked
```

- Close or retarget [#60](https://github.com/jose-compu/crisp/issues/60).
