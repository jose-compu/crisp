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

Also useful: [docs/KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md), [docs/ERROR_CATALOG.md](docs/ERROR_CATALOG.md). Web site: [jose-compu.github.io/crisp](https://jose-compu.github.io/crisp/) (source on the [`docs`](https://github.com/jose-compu/crisp/tree/docs) branch).

**Interop note:** known Rust `Result` APIs (`serde_json`, `ureq`) lower to Crisp ambient errors (`?` / `CrispError::Thrown`). Use `catch` or let the function stay fallible.

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
reveal types examples/hello  # see inferred signatures (see §10)
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

See `examples/math`, `examples/design_patterns`, `examples/enums`, `examples/vec2_methods` (inherent methods + nested mods).

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

`<path>` defaults to `.` (searches upward for `crisp.toml`).

| Command | What it does |
|---------|----------------|
| `crpc check <path>` | Fast analyze: resolve, typecheck, ownership probe — **no** Cargo build |
| `crpc emit <path>` | Write generated Rust crate to `<path>/target/rust/` and stop |
| `crpc build <path>` | Emit + `cargo build` (native binary via `rustc`) |
| `crpc run <path>` | Build and run the binary (day-to-day for apps/examples) |
| `crpc test <path>` | Emit + run Crisp `test` / `test_compile_fail` via `cargo test` |
| `crpc resolve <path>` | Print resolved module graph (debug) |
| `crpc parse <file.crp>` | Print AST for one file (debug) |

Pipeline order for a successful `run`/`test`: analyze → emit Rust under `target/rust/` → Cargo/`rustc`. Use `check` while editing; use `emit` when you want to read the generated Rust.
## 10. Inspect what the compiler inferred (`reveal`)

### What is `reveal`?

`crpc` **builds and runs** your project. `reveal` **explains** what the compiler decided behind the scenes.

Crisp source often omits types, borrows (`&` / `&mut`), lifetimes, and error sets — the compiler infers them. `reveal` prints those decisions so you can learn the language and debug surprises without opening `target/rust/` by hand.

| Tool | Job |
|------|-----|
| `crpc check` / `run` / `test` | “Does this compile and work?” |
| `reveal <subcommand>` | “What did inference emit / decide?” |
| `crpc emit` | Write the full generated Rust crate under `target/rust/` |

`reveal` ships next to `crpc` (same `cargo build -p crpc` / `cargo install --path crates/crpc`). Spec reference: §16.

### Try it (hello)

From the repo root (with `reveal` on `PATH`):

```bash
reveal types examples/hello      # inferred signatures
reveal ownership examples/hello  # & / &mut / owned per param
reveal rust examples/hello       # generated Rust entry file
reveal --help
```

Example — `reveal types examples/hello` prints something like:

```
greet(name: &str) -> str
main() -> ()
```

Your Crisp may only say `greet(name) = …`; `reveal` shows that `name` became `&str`.

### Common questions → which command

| I want to… | Run |
|------------|-----|
| See function signatures (types / `&`) | `reveal types <path>` or `reveal ownership <path>` |
| See the Rust `crpc` would generate | `reveal rust <path>` |
| See which errors a fallible fn can throw | `reveal errors <path>` |
| List `trait` / `impl Trait for` in a crate | `reveal traits <path>` (try `examples/show_trait`) |
| See lifetimes on parameters | `reveal lifetimes <path>` |
| See the sealed public API (`crisp.lock`) | `reveal seal <path>` |

`<path>` is a crate root (folder with `crisp.toml`) or any path inside that crate. Default is `.`.

### Full command list

| Command | Role | Status |
|---------|------|--------|
| `reveal types <path>` | Inferred signatures | Solid |
| `reveal ownership <path>` | Borrow / move / copy (+ rustc fallbacks) | Solid |
| `reveal lifetimes <path>` | Lifetime parameters | Solid |
| `reveal errors <path>` | Reachable `CrispError` sets | Solid |
| `reveal rust <path>` | Emitted Rust entry | Solid |
| `reveal seal <path>` | Sealed pub API (`crisp.lock`) | Solid |
| `reveal traits <path>` | User traits + impls (+ shape traits if any) | User traits + data shapes (`examples/shapes`, #61) |
| `reveal expand <path>` | Annotated Crisp outline | Shallow body stubs |
| `reveal diff <path>` | Crisp vs Rust names | Name-level summary only |
| `reveal map <path>` | Alloc / drop notes | Coarse CIR notes |

Gaps vs the draft spec: [KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md), [SPEC_IMPL_DELTA.md](docs/SPEC_IMPL_DELTA.md).

## 11. Language server (`crisp-lsp`)

Stdio LSP host (#56):

```bash
cargo install --path crates/crisp-lsp --locked
crisp-lsp   # speaks LSP on stdin/stdout
```

Capabilities today: `textDocument/hover`, `textDocument/inlayHint`, diagnostics on open/change/save (crate-level analyze). Library API remains `CrispAnalysis` for custom hosts.

```rust
use crisp_lsp::CrispAnalysis;
use std::path::Path;

let analysis = CrispAnalysis::analyze(Path::new("examples/hello"))?;
let hints = analysis.inlay_hints(Path::new("examples/hello/src/main.crp"))?;
```

### VS Code / Cursor extension

[`editors/vscode-crisp`](editors/vscode-crisp) — highlighting + optional LSP client:

```bash
./scripts/package-vsix.sh
# Extensions: Install from VSIX… → editors/vscode-crisp/*.vsix
cargo install --path crates/crisp-lsp --locked   # crisp-lsp on PATH
```

Dev symlink / F5: see [`editors/vscode-crisp/README.md`](editors/vscode-crisp/README.md).

## 12. Example projects

| Example | Topics |
|---------|--------|
| `hello` | Minimal program |
| `math` | Integer + float arithmetic, multi-module tests |
| `nested_math` | Nested `src/math/vector.crp` module tree |
| `vec2_methods` | Inherent `impl Vec2` + nested `math.vector` (§5.4 / #20) |
| `point_impl` | Flat inherent `impl Point` methods |
| `show_trait` | `trait Show` + `impl Show for Point` (§3.6 / #50) |
| `shapes` | Data `shape` → generated trait + structural calls (§3.5 / #61) |
| `loops` | `while` / `for` / `loop` + `break`/`continue` (§6.3) |
| `trait_defaults` | Trait default method bodies (§3.6 / #59) |
| `std_traits` | Prelude Show/Eq/Ord → Display/PartialEq/Ord (§15.4 / #27) |
| `net_http` | `parse_ip` + `ureq` GET via `rust = true` (§15.2 / #28) |
| `feature_gallery` | Nested mods + enums + inherent methods together |
| `rust_import` | Call `serde_json` via bare `use serde_json { from_str, to_string }` (§14.2 / #41) |
| `rust_shadow` | W0048 when Crisp module name collides with a Rust dep |
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
crates/                 Compiler workspace (`crpc`, emit, typeck, …)
docs/spec/              Language specification
editors/vscode-crisp/   `.crp` syntax highlighting (VS Code / Cursor)
examples/               Sample `.crp` projects
std/                    Standard library (Crisp)
tests/                  Integration fixtures
```

## Philosophy

> Explicit on demand, implicit by default.

Crisp lowers to a typed IR (CIR), emits explicit Rust, and delegates memory safety to `rustc`. When something is unclear, use `reveal` or `crpc emit` to see the generated code.

## Next steps

- Read the full spec: [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)
- Track milestones: [ROADMAP.md](ROADMAP.md)
- Run the workspace test suite: `cargo test --workspace --verbose`
