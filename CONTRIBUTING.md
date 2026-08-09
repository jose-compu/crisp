# Contributing to Crisp

Thank you for contributing. Crisp is a **Rust-hosted bootstrap** compiler that transpiles `.crp` to Rust. The formal language document is still **spec v0.2.0-draft**; see [KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md) and [SPEC_IMPL_DELTA.md](docs/SPEC_IMPL_DELTA.md).

## Prerequisites

- Rust **1.85+** (see `rust-version` in root `Cargo.toml`)
- `cargo`, `rustc`, and preferably `rustfmt` / `clippy` on `PATH`

## Setup

```bash
git clone https://github.com/jose-compu/crisp.git
cd crisp
./scripts/install-git-hooks.sh   # pre-commit / pre-push: cargo fmt --check
cargo build --release -p crpc
# binaries: target/release/crpc and target/release/reveal
export PATH="$PWD/target/release:$PATH"
crpc --version
```

Git hooks live in [`.githooks/`](.githooks/) (`core.hooksPath`). They mirror CI: `cargo fmt --all -- --check`.

Alternatively:

```bash
cargo install --path crates/crpc --locked
cargo install --path crates/crisp-reveal --locked
```

(`reveal` is a second binary from the `crpc` package: `cargo build --release -p crpc` produces `target/release/reveal`.)

## Development loop

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --verbose
```

Exercise examples:

```bash
crpc check examples/hello
crpc test examples/math
crpc run examples/float_demo
```

## Pull requests

1. Prefer an open issue from [milestone v1.1.1+](https://github.com/jose-compu/crisp/milestones) or [epic #1](https://github.com/jose-compu/crisp/issues/1).
2. Cross-reference the **spec section** (e.g. §7.6, §12) in the PR description.
3. Add or update tests under the relevant crate’s `tests/` (or examples e2e lists).
4. Keep `cargo fmt --check` and clippy `-D warnings` green.
5. Do not attribute authorship to AI tools in commit trailers.

## License

Contributions are dual-licensed under MIT OR Apache-2.0 (see [LICENSE](LICENSE)).

## Further reading

- [QUICKSTART.md](QUICKSTART.md)
- [docs/RELEASE.md](docs/RELEASE.md)
- [ROADMAP.md](ROADMAP.md)
