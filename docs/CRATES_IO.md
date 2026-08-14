# crates.io publish (v1.5.1)

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

## Publish order

Bottom-up so path deps resolve to registry versions:

1. `crisp-ast`
2. `crisp-lexer`, `crisp-manifest`, `crisp-diagnostics`
3. `crisp-parser`
4. `crisp-resolve`, `crisp-typeck`
5. `crisp-ownership`, `crisp-errors`, `crisp-regions`
6. `crisp-cir`
7. `crisp-rust-emit`
8. `crisp-reveal`
9. `crisp-lang`, `crisp-lsp`

Helper (dry-run by default):

```bash
./scripts/publish-crates.sh          # dry-run (first crate fully; later need prior on registry)
./scripts/publish-crates.sh --execute  # real publish in order (requires cargo login)
```

Dry-run for crates after `crisp-ast` fails until their Crisp deps exist on crates.io — that is normal. Use `--execute` for the ordered live publish.

## Preconditions

1. `[workspace.package] version` is **1.5.1** (or the tag you are shipping). Internal `[workspace.dependencies]` entries must include matching `version = "…"` alongside `path` so publish can rewrite to crates.io.
2. `cargo fmt` / `clippy -D warnings` / `cargo test --workspace` green.
3. Each package has `description`, `license`, `repository` (inherited from workspace).
4. Name availability: `crisp` / `crpc` / `crisp-cli` are **taken**; publish CLI as **`crisp-lang`** (bins `crisp` / `reveal`). Library `crisp-*` names checked available (2026-08-14). See [#68](https://github.com/jose-compu/crisp/issues/68).
5. `cargo login` with a token that can publish under the intended owners.

## After publish

- Tag `v1.5.1` and GitHub Release notes.
- README / QUICKSTART / docs site: prefer

```bash
cargo install crisp-lang --locked
cargo install crisp-lsp --locked
```

- Close or retarget [#60](https://github.com/jose-compu/crisp/issues/60).
