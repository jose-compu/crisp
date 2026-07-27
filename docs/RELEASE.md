# Release process

## Versioning

| Artifact | Policy |
|----------|--------|
| Workspace crates / `crpc` | Semver in root `Cargo.toml` (`1.1.0` today) |
| Language `edition` in `crisp.toml` | `2026` for spec v0.2; breaking language changes bump edition |
| `crisp.lock` | Regenerate when `pub` API signatures change |

GitHub milestones map upcoming work: **v1.1.1** (patch docs/CI), **v1.2.0** (language usability), **v1.3.0** (tooling/stdlib), **v2.0.0** (self-hosting).

## Checklist before tagging

1. Update [CHANGELOG.md](../CHANGELOG.md) (move Unreleased → version section).
2. Bump `[workspace.package] version` in root `Cargo.toml` (and lockfiles if any).
3. `cargo fmt --all --check`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace --verbose`
6. Spot-check: `crpc check/test/run` on `examples/hello`, `math`, `design_patterns`.
7. Create git tag `vX.Y.Z` and `gh release create` with changelog notes.

## Distribution decision (v1.x)

**Decision for the first public tags:** ship **source + GitHub Release notes**; document install via:

```bash
cargo install --path crates/crpc --locked
```

**crates.io:** deferred until crate packaging (binary names, readme, license files) is clean — revisit under [#29](https://github.com/jose-compu/crisp/issues/29). Prefer not to publish incomplete workspace crates early.

**Prebuilt binaries:** optional later (linux/macOS aarch64/x86_64) via `gh release` assets; not required for v1.1.1.

## Public-repo readiness checklist

- [ ] Flip repository visibility when maintainers agree
- [ ] Description, topics, homepage URL (docs site when live)
- [ ] CI badge on README pointing at `.github/workflows/ci.yml`
- [ ] Branch protection on `main` (reviews / status checks) if the org plan allows
- [ ] LICENSE dual files present (`LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`)
- [ ] SECURITY.md and CONTRIBUTING.md linked from README
- [ ] Close or defer all **v1.1.1** milestone issues
- [ ] Web docs: branch `docs`, folder `docs/` published via GitHub Pages (#39)

## Web docs branch

The documentation **website** lives only on branch **`docs`**, under folder **`docs/`** (not on `main`). Spec and markdown guides stay on `main` under `docs/spec/`, `docs/KNOWN_LIMITATIONS.md`, etc.

```bash
git fetch origin docs
git checkout docs
# site files: docs/index.html, docs/styles.css, …
```

GitHub Pages: Settings → Pages → Deploy from branch `docs` / folder `/docs`.
