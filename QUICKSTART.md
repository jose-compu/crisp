# Crisp Quickstart

Crisp (`.crp`) is a systems language that transpiles to Rust. You write compact source; `crpc` infers types, ownership, and error propagation, emits Rust, and `rustc` is the soundness boundary.

**Spec:** [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)

## Prerequisites

- [Rust](https://rustup.rs/) **1.85+** (MSRV; see workspace `rust-version` and `rust-toolchain.toml`) with `cargo` and `rustc` on `PATH`
- Clone this repository

## 1. Build / install the toolchain

```bash
cargo build --release -p crpc
export PATH="$PWD/target/release:$PATH"   # optional, for this shell
```

Or install into your Cargo bin directory (ships both `crpc` and `reveal`):

```bash
cargo install --path crates/crpc --locked
# ensure ~/.cargo/bin is on PATH
```

Verify:

```bash
crpc --version
reveal --version
```

Also useful: [docs/KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md), [docs/ERROR_CATALOG.md](docs/ERROR_CATALOG.md). Web site lives on the [`docs`](https://github.com/jose-compu/crisp/tree/docs) branch under `docs/`.

## 2. Run the hello example

```bash
crpc run examples/hello
```

Expected output:

```
"hello world"
```

Other useful commands:

```bash
crpc check examples/hello    # resolve + typecheck (fast)
crpc emit examples/hello     # write Rust to examples/hello/target/rust/
crpc build examples/hello    # emit + cargo build
crpc test examples/with_tests
```

## 3. Create a new project

Layout:

```
myapp/
  crisp.toml
  src/
    main.crp
```

**`crisp.toml`**

```toml
[package]
name = "myapp"
version = "0.1.0"
edition = "2026"

[build]
target = "rust"
runtime = "tokio"
error_model = "enum"
```

**`src/main.crp`**

```crisp
greet(name) = "hello " ++ name

pub main() = {
    msg := greet("world")
    print(msg)
}
```

Run from the project directory:

```bash
crpc run .
```

## 4. Language basics

### Functions and bindings

```crisp
double(n) = n + n

pub main() = {
    x := double(5)
    log("x={x}")
}
```

- `=` defines a function or binding.
- `:=` binds a local variable.
- `pub` exports an item from the module.
- `print` / `log` emit to stdout (via Rust `println!`).

### Strings and interpolation

```crisp
greet(name: str) = "hello " ++ name

pub main() = {
    who := "world"
    log("greet={greet(who)}")
}
```

Use `++` for concatenation. Interpolate with `"{expr}"` inside double-quoted strings.

### Comments

```crisp
-- single-line

{- block comment -}
```

### Types (when you need them)

Crisp infers types by default. Annotate parameters or fields when inference is ambiguous:

```crisp
port(host: str, n: int) = n
```

Primitive names: `int`, `uint`, `float`, `bool`, `char`, `str`.

## 5. Structs

```crisp
type ServerConfig = {
    host: str  = "127.0.0.1"
    port: uint = 9000
    debug: bool = false
}

pub main() = {
    cfg := ServerConfig { port: 3000 }
    log("port={cfg.port}")
}
```

Omitted fields use defaults. Field access works on locals and on parameters when the struct is unique for that field (`cfg.port`); if several structs share the field name, annotate the parameter.

See `examples/defaults`, `examples/inventory`.

## 6. Modules and imports

One file per module under `src/`. The file stem is the module name (`arith.crp` → `arith`).

**`src/arith.crp`**

```crisp
pub sum(a, b) = a + b

test "sum works" = {
    assert_eq(sum(1, 2), 3)
}
```

**`src/main.crp`**

```crisp
use arith { sum }

pub main() = {
    log("sum={sum(2, 3)}")
}
```

Modules in a flat `src/` directory may import each other in any filename order. Nested layouts work too (`src/math/vector.crp` → `use math.vector { … }`; see `examples/nested_math`).

See `examples/math`, `examples/design_patterns`, `examples/enums`.

## 7. Tests

```crisp
test "greet works" = {
    assert_eq(greet("world"), "hello world")
}

test_compile_fail "unknown name" = {
    definitely_not_a_builtin()
}
```

```bash
crpc test .
```

Runtime tests run via emitted Rust `#[test]` functions. `test_compile_fail` asserts that a fragment fails typechecking.

See `examples/with_tests`, `examples/math`.

## 8. Fallible functions

Mark functions that can fail with `! ErrorType` and `throw`:

```crisp
type IoError = { message: str }

read_file(path) -> str ! IoError = throw IoError { message: "not found" }

pub main() = {
    text := read_file("missing.txt") catch _ -> "default"
    print(text)
}
```

Fallible calls lower to `Result<T, CrispError>`. Use `catch` for recovery.

See `examples/fallible`, `examples/fallible_chain`.

## 9. `crpc` commands

| Command | Description |
|---------|-------------|
| `crpc check <path>` | Resolve, typecheck, ownership probe (no full build) |
| `crpc emit <path>` | Emit Rust to `<path>/target/rust/` |
| `crpc build <path>` | Emit + `cargo build` |
| `crpc run <path>` | Build and run the binary |
| `crpc test <path>` | Run `test` / `test_compile_fail` blocks |
| `crpc resolve <path>` | Print resolved module graph (debug) |
| `crpc parse <file.crp>` | Print AST (debug) |

`<path>` defaults to `.` (searches upward for `crisp.toml`).

## 10. Inspect what the compiler inferred (`reveal`, spec §16)

`reveal` is a second binary from the `crpc` package. It reconstructs precision that surface syntax omits.

```bash
reveal types examples/hello
reveal ownership examples/server
reveal rust examples/math
reveal --help
```

| Command | Role | Fidelity today |
|---------|------|----------------|
| `reveal types <path>` | Inferred signatures | Solid |
| `reveal ownership <path>` | Borrow / move / copy (+ §7.6 fallbacks) | Solid |
| `reveal lifetimes <path>` | Region / lifetime params | Solid |
| `reveal errors <path>` | Reachable `CrispError` sets | Solid |
| `reveal rust <path>` | Emitted Rust entry | Solid (crate-level) |
| `reveal seal <path>` | Sealed pub API (`crisp.lock`) | Solid |
| `reveal traits <path>` | Shape / trait summary | Limited (see #20 / #21) |
| `reveal expand <path>` | Annotated Crisp outline | Shallow body stubs |
| `reveal diff <path>` | Crisp vs Rust | Name-level summary, not full side-by-side |
| `reveal map <path>` | Alloc / drop notes | Coarse CIR notes, not span-accurate |

`<path>` is a crate root (directory with `crisp.toml`) or a path inside one. Details and gaps: [KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md), [SPEC_IMPL_DELTA.md](docs/SPEC_IMPL_DELTA.md).

## 11. Editor analysis API (`crisp-lsp`)

There is **no stdio LSP server** yet. The `crisp-lsp` crate exposes `CrispAnalysis` for hosts that want to wire their own protocol:

- `CrispAnalysis::analyze(path)` — full-crate pipeline
- `hover` / `inlay_hints` / `call_overlays` / `code_lenses` / `emitted_rust`

```rust
use crisp_lsp::CrispAnalysis;
use std::path::Path;

let analysis = CrispAnalysis::analyze(Path::new("examples/hello"))?;
let hints = analysis.inlay_hints(Path::new("examples/hello/src/main.crp"))?;
```

Until a stdio host ships (#18), use `reveal` and `crpc check` from the editor’s task runner.

## 12. Example projects

| Example | Topics |
|---------|--------|
| `hello` | Minimal program |
| `math` | Integer + float arithmetic, multi-module tests |
| `nested_math` | Nested `src/math/vector.crp` module tree |
| `float_demo` | `Vec2` geometry, `lerp`, circle metrics, `**` pow |
| `defaults` | Struct default fields |
| `fallible` | Error propagation + `catch` |
| `inventory` | Structs, multi-module domain model |
| `vec_ops` | `vec` (`new`, `push`, `len`) |
| `patterns` | `match` |
| `async_hello` | Async runtime |
| `ffi` | External Rust FFI |
| `sealed` | `crisp.lock` sealed API |
| `kitchen_sink` | Combined features |
| `design_patterns` | GoF-style patterns (multi-module) |
| `abnormal_suite` | Compile-fail edge cases |

Run any example:

```bash
crpc run examples/<name>
crpc test examples/<name>
```

## 13. Project layout reference

```
crates/          Compiler workspace (`crpc`, emit, typeck, …)
docs/spec/       Language specification
examples/        Sample `.crp` projects
std/             Standard library (Crisp)
tests/           Integration fixtures
```

## Philosophy

> Explicit on demand, implicit by default.

Crisp lowers to a typed IR (CIR), emits explicit Rust, and delegates memory safety to `rustc`. When something is unclear, use `reveal` or `crpc emit` to see the generated code.

## Next steps

- Read the full spec: [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)
- Track milestones: [ROADMAP.md](ROADMAP.md)
- Run the workspace test suite: `cargo test --workspace --verbose`
