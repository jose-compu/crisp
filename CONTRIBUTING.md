# Contributing to Crisp

Thank you for contributing. Crisp is a **Rust-hosted bootstrap** compiler that transpiles `.crp` to Rust. The formal language document is still **spec v0.2.0-draft**; see [KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md) and [SPEC_IMPL_DELTA.md](docs/SPEC_IMPL_DELTA.md).

## Prerequisites

- Rust **1.85+** (MSRV in `[workspace.package] rust-version`; local default via `rust-toolchain.toml` → stable)
- `cargo`, `rustc`, and preferably `rustfmt` / `clippy` on `PATH`
- CI: Ubuntu + macOS on stable, plus an Ubuntu **MSRV 1.85** job (`.github/workflows/ci.yml`)

## Setup

```bash
git clone https://github.com/jose-compu/crisp.git
cd crisp
./scripts/install-git-hooks.sh   # pre-commit / pre-push: cargo fmt --check
cargo build --release -p crpc
# binaries: target/release/crpc and target/release/reveal
export PATH="$PWD/target/release:$PATH"
crpc --version
reveal --version
```

`crpc` builds/runs Crisp crates. `reveal` prints what inference decided (types, ownership, emitted Rust, …). Beginner guide: [QUICKSTART §10](QUICKSTART.md#10-inspect-what-the-compiler-inferred-reveal).

Git hooks live in [`.githooks/`](.githooks/) (`core.hooksPath`). They mirror CI: `cargo fmt --all -- --check`.

Alternatively:

```bash
cargo install --path crates/crpc --locked
# installs both `crpc` and `reveal` (do not `cargo install` crates/crisp-reveal — library only)
```

## Development loop

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --verbose
```

### Updating emit / CIR snapshots (#25)

Pinned outputs for `examples/hello` and `examples/math` live under
`crates/crisp-rust-emit/tests/snapshots/` (`emit_hello_main`, `emit_math_bundle`,
`cir_outline_*`). After intentional emit or CIR changes:

```bash
INSTA_UPDATE=1 cargo test -p crisp-rust-emit --test emit_pipeline
```

Review the `.snap` diffs before committing.

Exercise examples:

```bash
crpc check examples/hello
crpc test examples/math
crpc run examples/float_demo
```

## Pull requests

1. Prefer an open issue from [milestone v1.5.0+](https://github.com/jose-compu/crisp/milestones) or label `epic:publication`.
2. Cross-reference the **spec section** (e.g. §7.6, §12) in the PR description.
3. Add or update tests under the relevant crate’s `tests/` (or examples e2e lists).
4. Keep `cargo fmt --check` and clippy `-D warnings` green.
5. Do not attribute authorship to AI tools in commit trailers.

## Public preview notes

- Install: `cargo install --path crates/crpc --locked` until crates.io ([#66](https://github.com/jose-compu/crisp/issues/66); see [docs/CRATES_IO.md](docs/CRATES_IO.md)).
- Site: https://crisp-lang.org/
- Maintainers: enable branch protection on `main` (require CI) when the GitHub plan allows; flip visibility via [#58](https://github.com/jose-compu/crisp/issues/58).
- Release checklist: [docs/RELEASE.md](docs/RELEASE.md).

## License

Contributions are dual-licensed under MIT OR Apache-2.0 (see [LICENSE](LICENSE)).

## Further reading

- [QUICKSTART.md](QUICKSTART.md)
- [docs/RELEASE.md](docs/RELEASE.md)
- [ROADMAP.md](ROADMAP.md)
