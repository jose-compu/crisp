# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 1.8.x   | Yes |
| 1.7.x   | Best effort |
| 1.6.x   | Best effort |
| < 1.6   | No |

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security-sensitive reports.

Prefer one of:

1. **GitHub Security Advisories** for [jose-compu/crisp](https://github.com/jose-compu/crisp) (private report), or
2. Email the maintainers via the contact listed on the GitHub organization / repository profile.

Include:

- Crisp / `crisp` version (`crisp --version`)
- Minimal `.crp` repro if applicable
- Impact (wrong code gen, sandbox escape in emitted Rust tooling, etc.)

We aim to acknowledge reports within 7 days.

## Scope notes

Crisp lowers to Rust and relies on **`rustc` as the soundness boundary**. Issues that are pure Rust/borrow-checker failures on *correctly* emitted code are treated as compiler bugs (`crisp`), not as “Crisp is memory-unsafe by design.” Generated `unsafe` / FFI examples are intentional and require the same care as Rust `unsafe`.
