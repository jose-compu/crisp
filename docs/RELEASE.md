# Release process

## Versioning

| Artifact | Policy |
|----------|--------|
| Workspace crates / `crpc` | Semver in root `Cargo.toml` (`1.5.0` today) |
| Language `edition` in `crisp.toml` | `2026` for spec v0.2; breaking language changes bump edition |
| `crisp.lock` | Regenerate when `pub` API signatures change |
| MSRV | Rust **1.85** (`rust-version`); CI MSRV job + multi-OS (see `ci.yml`) |

GitHub milestones: **v1.5.0** (first public release), **v2.0.0** (self-hosting).

## Checklist before tagging

1. Update [CHANGELOG.md](../CHANGELOG.md) (move Unreleased → version section).
2. Bump `[workspace.package] version` in root `Cargo.toml`.
3. `cargo fmt --all --check`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace --verbose`
6. Spot-check: `crpc check/test/run` on `examples/hello`, `math`, `design_patterns`.
7. Create git tag `vX.Y.Z` and `gh release create` with changelog notes.

## Distribution decision (v1.x)

**Decision for public preview tags:** ship **source + GitHub Release notes**; document install via:

```bash
cargo install --path crates/crpc --locked
```

**crates.io:** deferred — revisit under [#60](https://github.com/jose-compu/crisp/issues/60). Prefer not to publish incomplete workspace crates early.

**Prebuilt binaries:** optional later (linux/macOS aarch64/x86_64) via `gh release` assets; not required for v1.4.x.

## Public-repo readiness checklist

| Item | Status |
|------|--------|
| Repo description + topics | Done (GitHub settings) |
| Homepage → GitHub Pages | Done — https://jose-compu.github.io/crisp/ |
| CI status badge on README | Done |
| LICENSE dual files + SECURITY + CONTRIBUTING linked | Done |
| Known limitations honest for current version | Done (v1.5.0) |
| Web docs on `docs` branch / Pages | Done (#39) |
| Branch protection on `main` | Maintainer action (Settings → Branches) if plan allows |
| Flip visibility to **public** | Maintainer action — [#58](https://github.com/jose-compu/crisp/issues/58) |

## Web docs branch

The documentation **website** lives on branch **`docs`**, under folder **`docs/`**. Spec and markdown guides stay on `main` under `docs/spec/`, `docs/KNOWN_LIMITATIONS.md`, etc.

Live site: https://jose-compu.github.io/crisp/

```bash
git fetch origin docs
git checkout docs
# site files: docs/index.html, docs/styles.css, …
```

GitHub Pages: Settings → Pages → Deploy from branch `docs` / folder `/docs`.
