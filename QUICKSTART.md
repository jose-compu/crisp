# Crisp Quickstart

Crisp (`.crp`) is a systems language that transpiles to Rust. You write compact source; `crisp` infers types, ownership, and error propagation, emits Rust, and `rustc` is the soundness boundary.

**Spec:** [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)

## Prerequisites

- [Rust](https://rustup.rs/) **1.85+** (MSRV) with `cargo` and `rustc` on `PATH`
- Optional: this repository (examples and contributing)

## 1. Install the toolchain

From crates.io (recommended — ships `crisp` and `reveal`):

```bash
cargo install crisp-lang --locked
crisp --version
reveal --version
```

From a clone of this repo (contributors / unreleased commits):

```bash
cargo build --release -p crisp-lang
export PATH="$PWD/target/release:$PATH"
# or: cargo install --path crates/crpc --locked
```

Also useful: [docs/KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md), [docs/ERROR_CATALOG.md](docs/ERROR_CATALOG.md). Web site: [crisp-lang.org](https://crisp-lang.org/) (source on the [`docs`](https://github.com/jose-compu/crisp/tree/docs) branch).

**Interop note:** known Rust `Result` APIs (`serde_json`, `ureq`) lower to Crisp ambient errors (`?` / `CrispError::Thrown`). Use `catch` or let the function stay fallible.

## 2. Run hello immediately

Create a tiny project (no repo clone required):

```bash
mkdir -p hello/src && cd hello
```

`crisp.toml`:

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2026"

[build]
target = "rust"
runtime = "tokio"
error_model = "enum"
```

`src/main.crp`:

```crisp
shape Named = {
    name: str
}

type Guest = {
    name: str = "world"
}

id(x: T) = x

greet(who: Named) = "hello {who.name}"

pub main() = {
    world := Guest {}
    print(id(greet(world)))
}
```

```bash
crisp run .
```

Expected:

```
"hello world"
```

Or, from a clone of this repository:

```bash
crisp run examples/hello
```

Other useful commands:

```bash
reveal types examples/hello  # see inferred signatures (see §10)
crisp check .                 # resolve + typecheck (fast)
crisp emit .                  # write Rust under target/rust/
crisp build .                 # emit + cargo build
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
shape Named = {
    name: str
}

type Guest = {
    name: str = "world"
}

id(x: T) = x

greet(who: Named) = "hello {who.name}"

pub main() = {
    world := Guest {}
    print(id(greet(world)))
}
```

Run from the project directory:

```bash
crisp run .
```

## 4. Language basics

### Functions and bindings

```crisp
double(n) = n + n
apply(f, x) = f(x)
run(f) = f(21)
label(g, p) = g(p)

type Person = { name: str }

pub main() = {
    x := double(5)
    inc := |n| n + 1
    log("x={x} next={inc(x)} twice={apply(_ * 2, x)}")
    log("trail={run { |x| x * 2 }} field={label(.name, Person { name: "Ada" })}")
}
```

- `=` defines a named function (the same kind of value as `|x| …`).
- `:=` binds a local variable (including a function value: `inc := |n| n + 1`).
- Pass a named item where a function is expected: `apply(double, 21)`.
- Sugar: holes (`apply(_ * 2, n)`), trailing last-arg (`run { |x| x * 2 }`), field/method sections (`.name`, `.magnitude()`). See `examples/closures`.
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

**Prefer implicit generics.** Unbound names in type position are parameters. Write `<>` only to pin a definition or to apply arguments:

```crisp
type Pair = { left: A, right: B }
id(x: T) = x
first(p: Pair<A, B>) = p.left

shape Boxy = { value: T }
unwrap_int(b: Boxy<int>) = b.value

trait Wrapper = { unwrap(self) -> T }
impl Wrapper for IntBox = { unwrap(self) = self.value }

label(u: HasName + HasId) = u.name

-- pins (same meaning, use when you want the binder visible):
type Pair<A, B> = { left: A, right: B }
id<T>(x: T) = x
```

A name that is already a type (`int`, `Pair`, …) is that type, not a parameter. An explicit `<T>` that shadows a type is an error (E0049).

Unannotated `id(x) = x` is a scheme when the body leaves type variables free. Crate-internal items used at one concrete type emit monomorphic Rust; `pub` items stay schemes and are sealed in `crisp.lock`. `reveal types` shows the emitted bound (`id<T: Clone>(x: T) -> T`). Locals and `mut` bindings are not generalized.

Start with `examples/generics_implicit`. Also `examples/generics` (explicit pins), `examples/generics_pub`, `examples/shapes_generic`, and `examples/shapes_user` (user `Measure` on `T`). `where` clauses and rich written `T: Show` bounds are still limited.

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
crisp test .
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

## 9. `crisp` commands

`<path>` defaults to `.` (searches upward for `crisp.toml`).

| Command | What it does |
|---------|----------------|
| `crisp check <path>` | Fast analyze: resolve, typecheck, ownership probe — **no** Cargo build |
| `crisp emit <path>` | Write generated Rust crate to `<path>/target/rust/` and stop |
| `crisp build <path>` | Emit + `cargo build` (native binary via `rustc`) |
| `crisp run <path>` | Build and run the binary (day-to-day for apps/examples) |
| `crisp test <path>` | Emit + run Crisp `test` / `test_compile_fail` via `cargo test` |
| `crisp resolve <path>` | Print resolved module graph (debug) |
| `crisp parse <file.crp>` | Print AST for one file (debug) |

Pipeline order for a successful `run`/`test`: analyze → emit Rust under `target/rust/` → Cargo/`rustc`. Use `check` while editing; use `emit` when you want to read the generated Rust.
## 10. Inspect what the compiler inferred (`reveal`)

### What is `reveal`?

`crisp` **builds and runs** your project. `reveal` **explains** what the compiler decided behind the scenes.

Crisp source often omits types, borrows (`&` / `&mut`), lifetimes, and error sets — the compiler infers them. `reveal` prints those decisions so you can learn the language and debug surprises without opening `target/rust/` by hand.

| Tool | Job |
|------|-----|
| `crisp check` / `run` / `test` | “Does this compile and work?” |
| `reveal <subcommand>` | “What did inference emit / decide?” |
| `crisp emit` | Write the full generated Rust crate under `target/rust/` |

`reveal` ships next to `crisp` (same `cargo build -p crisp-lang` / `cargo install --path crates/crpc`). Spec reference: §16.

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
greet(who: Named) -> str  -- used as Named
id<T: Clone>(x: T) -> T  -- used as str
main() -> ()
```

Your Crisp may only say `id(x: T)` and `greet(who: Named)`; `reveal` shows the inferred scheme and that `Named` is a shape bound.

### Common questions → which command

| I want to… | Run |
|------------|-----|
| See function signatures (types / `&`) | `reveal types <path>` or `reveal ownership <path>` |
| See the Rust `crisp` would generate | `reveal rust <path>` |
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
| `hello` | Shapes, implicit generics, field defaults |
| `math` | Integer + float arithmetic, multi-module tests |
| `nested_math` | Nested `src/math/vector.crp` module tree |
| `vec2_methods` | Inherent `impl Vec2` + nested `math.vector` (§5.4 / #20) |
| `point_impl` | Flat inherent `impl Point` methods |
| `show_trait` | `trait Show` + `impl Show for Point` (§3.6 / #50) |
| `shapes` | Data `shape` → generated trait + structural calls (§3.5 / #61) |
| `generics_implicit` | Preferred: free type names as binders — no `<T>` on defs (#75) |
| `generics` | Explicit `<>` pins + parametric shapes (#70 / #71) |
| `generics_pub` | Unannotated `id(x)=x` — internal mono vs `pub` scheme (#76) |
| `shapes_generic` | `shape Boxy`, `shape HasPosition` + squared Euclidean distance (#70) |
| `shapes_user` | `HasPosition<T>` + inferred user `Measure` bound (not int/float `+`) (#84) |
| `loops` | `while` / `for` / `loop` + `break`/`continue` (§6.3) |
| `closures` | Function values, holes, trailing last-arg, field/method sections (#72 / #87–#89) |
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
crisp run examples/<name>
crisp test examples/<name>
```

## 13. Project layout reference

```
crates/                 Compiler workspace (`crisp`, emit, typeck, …)
docs/spec/              Language specification
editors/vscode-crisp/   `.crp` syntax highlighting (VS Code / Cursor)
examples/               Sample `.crp` projects
std/                    Standard library (Crisp)
tests/                  Integration fixtures
```

## Philosophy

> Explicit on demand, implicit by default.

Crisp lowers to a typed IR (CIR), emits explicit Rust, and delegates memory safety to `rustc`. When something is unclear, use `reveal` or `crisp emit` to see the generated code.

## Next steps

- Read the full spec: [docs/spec/CrispLang-SPECS-0.2.0.md](docs/spec/CrispLang-SPECS-0.2.0.md)
- Track milestones: [ROADMAP.md](ROADMAP.md)
- Run the workspace test suite: `cargo test --workspace --verbose`
