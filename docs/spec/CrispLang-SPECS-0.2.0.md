# The Crisp Programming Language Specification

**Version:** 0.2.0-draft
**Status:** Abstract Design Specification
**File Extension:** `.crp`
**Compiler / Transpiler:** `crisp`
**Toolchain:** `reveal`
**Target:** Rust (source-to-IR-to-Rust); `rustc` is the final source of truth.

---

## 0. Changes from v0.1.0

This version retargets Crisp from a direct-to-LLVM language to a **transpiler that
produces correct, compilable Rust**. This single decision reshapes the semantics:
several v0.1.0 guarantees are now *delegated* rather than *implemented*, and some
v0.1.0 features are constrained to remain expressible in Rust.

Summary of material changes:

- **§0.1 Compilation model.** Crisp lowers to a typed intermediate representation
  (CIR) and emits Rust from it. `rustc` then compiles the Rust. A `rustc` error on
  generated code is defined to be a **Crisp compiler bug**, not a user error.
- **§0.2 Whole-program compilation.** Ownership is inferred globally (per the v0.1.0
  ambition). The consequence — made explicit here — is that **the unit of compilation
  is the whole program (or a sealed crate)**. Separate compilation with a stable ABI
  is not supported, because inferred signatures depend on call sites.
- **§0.3 Uniform error type.** Ambient error propagation is retained, but all fallible
  functions lower to `Result<T, CrispError>` where `CrispError` is a single
  program-global generated enum. The per-function error sets shown by `reveal` are
  *documentation computed by analysis*, not distinct Rust types.
- **§0.4 Soundness is borrowed.** Crisp performs its own borrow/region analysis for the
  sole purpose of producing good diagnostics at the Crisp level. It is **not** the
  soundness boundary. Memory-safety guarantees hold *because the generated Rust
  compiles*, not because Crisp's checker is independently sound.
- **§0.5 Notation fixes.** Resolved v0.1.0 collisions: the bottom type is now `Never`
  (was `never`, colliding with the `!never` error annotation); Option `?T` and
  `Result` roles are disambiguated (§9, §15); removed the duplicate `type Vec3`
  example from the module section.
- **§0.6 Inference determinism.** The v0.1.0 claim of "Hindley-Milner + ownership" is
  refined: type inference is HM-style with constraint solving, but ownership inference
  is a **separate, deterministic dataflow pass** over the typed program (§7). The two
  are not unified into one inference algorithm, because affine ownership does not
  compose with HM unification.

A migration note for each breaking change appears in Appendix C.

---

## 1. Philosophy

Crisp is a systems language whose source is optimized for writing and local reading,
and whose semantics are defined by **lowering to Rust**. The core principle is
unchanged:

> **Explicit on demand, implicit by default.**

What changed in v0.2.0 is the meaning of "implicit." In v0.1.0 the compiler *was* the
authority on safety. In v0.2.0 the compiler is a **front end that produces Rust**, and
Rust is the authority. Crisp's job is to infer the annotations a Rust programmer would
have written, emit them, and let `rustc` verify them. The `reveal` toolchain surfaces
everything the inference computed.

### 1.1 Design Goals

- Memory safety and the absence of data races — **inherited from the generated Rust**,
  verified by `rustc` at build time.
- Type safety through HM-style inference; annotations optional unless ambiguous.
- Ownership and borrowing inferred from usage by a deterministic global dataflow pass,
  then **emitted as explicit Rust** (`&`, `&mut`, owned, `Clone`, etc.).
- Lifetimes inferred where Rust elision rules cover them; emitted explicitly where they
  do not.
- Ambient error propagation, lowered to a uniform `Result<T, CrispError>`.
- Compact syntax with no ceremonial keywords.
- Tooling-first: `reveal` reconstructs the precision the surface syntax omits and the
  uniform error type erases.

### 1.2 Non-Goals

- Crisp is not a scripting language. It transpiles to Rust, which compiles to native code.
- Crisp does not add a garbage collector. Shared ownership uses Rust's `Rc`/`Arc`,
  explicitly (§8.7).
- Crisp does not provide a stable library ABI. See §0.2 / §12.5.
- Crisp's own analysis is **not** claimed to be independently sound (§0.4). It defers to
  `rustc`.

---

## 2. Lexical Structure

Unchanged from v0.1.0 except where noted. Reproduced for completeness.

### 2.1 Source Encoding

All Crisp source files are UTF-8 encoded.

### 2.2 Comments

```rust
-- single-line comment

{- block comment -}

{- block comments {- can nest -} -}
```

### 2.3 Identifiers

Letters, digits, underscores; must begin with a letter or underscore. A leading `_`
suppresses unused-variable warnings.

```rust
valid_name
_private
Point3D
```

Identifiers that are Rust keywords (`fn`, `let`, `impl`, `move`, `dyn`, `async`, …) are
legal in Crisp and are **raw-escaped** on emission (`r#name`). The transpiler owns this
mapping; the programmer never sees it.

### 2.4 Keywords

```rust
if then else match with for in while do loop
return break continue
type trait shape impl
own mut ref shared rc arc
async await spawn
use as mod pub
true false none some
catch throw panic
unsafe extern test
```

(`loop`, `arc`, `panic`, `unsafe`, `extern`, `test` are now reserved words; in v0.1.0
several of these were contextual. Reserving them avoids parse ambiguities that
complicated IR lowering.)

### 2.5 Operators

```rust
Arithmetic:    +  -  *  /  %  **
Comparison:    == != < > <= >=
Logical:       && || !
Bitwise:       &&& ||| ^^^ ~ << >>      (see note)
Assignment:    = += -= *= /= %=
Binding:       :=  mut:=
Range:         ..  ..=
Concatenation: ++
Pipe:          |>
Constraint:    +  (in type position only)
Access:        .
Try (explicit):?                         (postfix; see §9.7)
```

**Note (changed from v0.1.0):** bitwise AND/OR/XOR are now `&&&`/`|||`/`^^^`. In v0.1.0
they were `&`/`|`/`^`, which collided with: borrow `&` in type position, the constraint
`+`… and especially the enum `|` and the error-union `|`. Disambiguating bitwise ops by
spelling removes a class of parser hacks. `**` is exponentiation and lowers to
`.pow()` / `.powf()` as appropriate.

### 2.6 Delimiters

```rust
{ }    -- blocks, struct literals, trait/shape bodies
( )    -- grouping, tuples, arguments
[ ]    -- array/slice literals, indexing
< >    -- explicit generic parameters (rare)
```

### 2.7 Literals

```rust
42      1_000_000      0xFF      0b1010      0o77
3.14    1.0e-10        1_000.5
"hello"                "interp: {name} is {age}"
r"raw: no \escaping"
"""
  multi-line {interpolation}
"""
'a'     '\n'    '\u{1F600}'
true    false
()
```

String interpolation lowers to `format!`. Raw strings lower to Rust raw strings. The
multi-line form lowers to a `format!` over a de-indented literal.

---

## 3. Type System

### 3.1 Primitive Types

| Crisp | Rust | Description |
|------|------|-------------|
| `int` | `i64` | Platform default signed (fixed at i64 for output stability) |
| `uint` | `u64` | Platform default unsigned |
| `i8..i128` / `u8..u128` | same | Fixed-width integers |
| `float` | `f64` | 64-bit float |
| `f32` | `f32` | 32-bit float |
| `bool` | `bool` | Boolean |
| `char` | `char` | Unicode scalar |
| `str` | `String` | **Owned** UTF-8 string |
| `&str` | `&str` | Borrowed string slice |
| `()` | `()` | Unit |
| `Never` | `!` | Bottom type (renamed from `never`; lowers to Rust `!` / diverging) |

**Changed:** `int`/`uint` are pinned to `i64`/`u64` rather than "platform-sized," so
generated Rust is deterministic across targets. Programmers wanting target-sized
integers use `isize`/`usize` (now exposed directly).

### 3.2 Compound Types

```rust
pair:  (int, str)        = (42, "hello")     -- tuple        -> (i64, String)
arr:   [int; 5]          = [1,2,3,4,5]        -- array        -> [i64; 5]
slice: [int]             = arr[1..4]          -- slice view   -> &[i64]
vec                       := vec![1,2,3]       -- growable     -> Vec<i64>
map                       := map!{"k" -> "v"} -- map          -> HashMap<...>
x:     ?int              = some(42)           -- option       -> Option<i64>
f:     (int,int) -> int                       -- fn type      -> fn(i64,i64)->i64 / Fn
```

`?T` is the **only** spelling of optionality (lowers to `Option<T>`). `Result` is not
written by users for the *common* fallible path — that is the ambient error system
(§9). `Result` appears in user code only when interoperating with explicit Rust APIs
(§14), where it is spelled `Result<T, E>` and lowers identically.

### 3.3 User-Defined Types

#### 3.3.1 Structs

```rust
type Point = { x: float, y: float }
p := Point { x: 1.0, y: 2.0 }

type Config = {
    host: str  = "localhost"
    port: uint = 8080
    debug: bool = false
}
cfg := Config { debug: true }   -- other fields use defaults
```

Field defaults lower to a generated `Default`-like constructor plus a builder, because
Rust struct literals cannot omit fields. The transpiler emits an associated
`Config::with(...)` that fills defaults; `Config { debug: true }` lowers to a call to
it. (v0.1.0 left this mechanism unspecified.)

#### 3.3.2 Enums

```rust
type Color = | Red | Green | Blue | Custom(u8, u8, u8)

type List<T> = | Nil | Cons(T, List<T>)
```

`Result<T,E>` is **no longer a user-facing built-in enum to redeclare** (it was listed
twice across v0.1.0 §3.3.2 and §15.1). It exists for FFI/interop and is provided by the
prelude.

Recursive types (`List<T>`) lower with `Box` inserted automatically at the recursion
point (`Cons(T, Box<List<T>>)`), since Rust requires indirection for recursive enums.
`reveal types` shows where boxing was inserted.

#### 3.3.3 Type Aliases

```rust
type Name     = str
type Callback = (int) -> bool
type Matrix   = vec<vec<float>>
```

Lower to Rust `type` aliases.

### 3.4 Type Inference

Crisp uses **HM-style inference with constraint solving** for types, run *before* the
ownership pass (§0.6). Inference covers local bindings, parameter types, return types,
generic parameters, and trait bounds.

```rust
add(x, y) = x + y
-- reveal types:  add<T: Add<Output = T>>(x: T, y: T) -> T
```

The emitted Rust carries the inferred bound explicitly:

```rust
fn add<T: std::ops::Add<Output = T>>(x: T, y: T) -> T { x + y }
```

Explicit annotations are accepted everywhere and act as constraints. The compiler
requests an annotation only when a type variable is left genuinely ambiguous *and* the
choice is observable in the generated Rust.

> **Interaction note (new).** Type inference does not see ownership; ownership inference
> (§7) runs afterward on the fully-typed program. This ordering is why the two are
> described as separate passes rather than one algorithm. A consequence: a type
> annotation never changes an ownership decision, and vice versa.

### 3.5 Shapes (Structural Types)

Shapes are structural constraints satisfied automatically by any matching type.

```rust
shape HasPosition = { x: float, y: float }

distance(a: HasPosition, b: HasPosition) -> float = {
    dx := a.x - b.x
    dy := a.y - b.y
    sqrt(dx*dx + dy*dy)
}
```

**Lowering (newly specified — this was the largest gap in v0.1.0).** Rust has no
structural types. A shape lowers to a **generated trait plus blanket-style impls**:

- `shape HasPosition` emits a trait `HasPosition` with accessor methods
  (`fn x(&self) -> f64; fn y(&self) -> f64;`).
- For every concrete type used at a shape-typed call site, the transpiler emits an impl
  of that trait providing the accessors (field reads for data shapes, method forwards
  for method shapes).
- The function becomes generic: `fn distance<A: HasPosition, B: HasPosition>(...)`.

Because impls are generated only for types that actually reach the call site, this
requires whole-program knowledge — consistent with §0.2. `reveal traits` lists the
generated shape impls.

Method-requiring shapes:

```rust
shape Measurable = { len(self) -> uint }
```

Anonymous shapes (`a: { x: float, y: float }`) lower the same way, with a compiler-named
trait.

Parametric (generic) shapes use the same `<>` type parameters as types and functions:

```rust
shape Boxy<T> = { value: T }

unwrap_int(b: Boxy<int>) = b.value
```

This lowers to a generic Rust trait (`trait Boxy<T> { fn value(&self) -> T; }`) plus
structural impls (`impl Boxy<i64> for IntBox`). A bare `shape HasPosition` (no
parameters) is unchanged.

### 3.6 Traits (Nominal Types)

Traits are explicit semantic contracts and map directly to Rust traits.

```rust
trait Show = { show(self) -> str }

trait Comparable = {
    compare(self, other: self) -> Ordering
    less_than(self, other: self) -> bool = self.compare(other) == Ordering.Less
}
```

`self` lowers to `&self` by default and to `self`/`&mut self` when the ownership pass
determines the method consumes or mutates the receiver (§7). Default methods lower to
default trait method bodies.

#### 3.6.1 Implementation

```rust
type Point = { x: float, y: float }
impl Show for Point = { show(self) = "({self.x}, {self.y})" }
```

Body-less shorthand opt-in (type already has the methods as free functions):

```rust
serialize(self: Point) -> bytes = encode_json(self)
deserialize(b: bytes) -> Point  = decode_json(b)
impl Serializable for Point      -- compiler verifies signatures, emits the impl
```

#### 3.6.2 Satisfaction Rules

- A type satisfies a trait **only** through an explicit `impl`.
- Matching methods are necessary but not sufficient.
- The compiler *suggests* `impl` on structural match but never inserts it.

#### 3.6.3 Shape vs. Trait

| Property | Shape | Trait |
|----------|-------|-------|
| Matching | Structural (automatic) | Nominal (explicit `impl`) |
| Rust lowering | Generated trait + generated impls | Direct Rust trait + your impls |
| Default methods | No | Yes |
| Opt-in | None | Required |
| Coherence risk | Compiler-managed | Standard Rust orphan rules apply |

> **Coherence caveat (new).** Generated shape impls can collide with Rust's orphan and
> overlap rules once FFI types are involved. The transpiler resolves shapes via
> newtype wrappers when a direct impl would violate coherence; `reveal traits` flags
> any wrapper it introduced.

### 3.7 Combined Constraints

```rust
save_named(item: { name: str } + Serializable) = {
    log("saving {item.name}")
    fs.write("{item.name}.bin", item.serialize())
}
```

Lowers to a generic bound combining the generated shape trait and the nominal trait:
`fn save_named<T: HasName + Serializable>(item: T) { ... }`.

### 3.8 Generics

Inferred from usage; explicit `<>` when needed. The same brackets are used on
`type`, functions, `trait`, and `shape` (see §3.5 for parametric shapes).

Unbound names in type position are parameters. A name that is already a type
(prelude, `type` / `shape` / `trait` in scope) is that type. An explicit binder
that shadows a type is an error. `<>` on a definition is a pin.

```rust
identity(x) = x
-- reveal: identity<T>(x: T) -> T

id(x: T) = x
type Pair = { left: A, right: B }
first(p: Pair<A, B>) = p.left
shape Boxy = { value: T }
trait Wrapper = { unwrap(self) -> T }

-- pins (equivalent):
id<T>(x: T) = x
type Pair<A, B> = { left: A, right: B }

impl Wrapper for IntBox = { unwrap(self) = self.value }
-- pin: impl Wrapper<int> for IntBox = { … }

label(u: HasName + HasId) = u.name

convert<T, U>(x: T) -> U where T: Into<U> = x.into()
```

Applications use the same `<>` (`Pair<int, str>`, `Boxy<int>`, `Wrapper<int>`).
`where` clauses lower verbatim to Rust when present; the bootstrap currently
ships explicit params without `where` / HRTB.

Polymorphism is a **publication artifact**. After checking a function item:

- leftover free type variables become a scheme (`identity(x) = x` → `identity<T>(x: T) -> T`);
- a crate-internal scheme used at only one concrete type is monomorphized for emit;
- a crate-internal scheme used at several types stays generic;
- a `pub` scheme is never monomorphized — it is frozen in `crisp.lock`. Changing
  that sealed `rust_signature` (including a new hidden `T: Clone` bound) without
  an explicit pin is **E0080**. `id<T>` / `pub id(x: int)` remain pins.

Only **function items** are generalized. Locals and `mut` bindings are not
(value restriction). Empty containers stay ambiguous until annotated.

`reveal types` shows the emit-grade scheme, including the hidden Clone bound
that every user generic carries on the way to Rust (`id<T: Clone>(x: T) -> T`),
and may list instantiating call sites. The lock `rust_signature` is that
contract.

---

## 4. Bindings and Mutability

### 4.1 Immutable Bindings

```rust
x := 42
name := "crisp"
```

Lowers to `let x = 42;`. Immutable by default (Rust's default).

### 4.2 Mutable Bindings

```rust
counter mut:= 0
counter = counter + 1
list mut:= vec![1,2,3]
list.push(4)
```

`mut:=` lowers to `let mut`. Reassigning an immutable binding is a Crisp-level error and
would also be a `rustc` error — Crisp reports it first for a better message.

### 4.3 Destructuring

```rust
(a, b)            := (1, 2)
{ x, y }          := Point { x: 1.0, y: 2.0 }
{ name, ..rest }  := config
[first, ..tail]   := vec![1,2,3,4,5]
```

Struct/tuple destructuring lowers to Rust patterns. Slice rest-patterns (`..tail`) lower
to Rust slice patterns where the binding is owned/borrowed per the ownership pass.

### 4.4 Constants

```rust
MAX_SIZE = 1024
PI       = 3.14159265358979
APP_NAME = "crisp-app"
```

Lower to `const` where the value is a Rust const-expr, else to a `once_cell`/`LazyLock`
static. The transpiler chooses; `reveal expand` shows which.

---

## 5. Functions

### 5.1 Definition

```rust
add(x, y) = x + y

sort(list, cmp) = {
    pivot := list.head
    less  := list.tail.filter(|x| cmp(x, pivot))
    more  := list.tail.filter(|x| !cmp(x, pivot))
    sort(less, cmp) ++ [pivot] ++ sort(more, cmp)
}
```

Last expression is the return value (lowers to a tail expression with no `;`). `return`
is available for early exit and lowers to Rust `return`.

### 5.2 Visibility

```rust
pub add(x, y) = x + y     -- pub fn
helper(x)     = x * 2     -- private fn
```

`pub` items form the **sealed boundary** of a crate (§12.5): their inferred signatures
are pinned in a lockfile so the crate can be depended upon without re-running global
inference at every downstream build. An unannotated `pub` function that still has
free type variables is recorded as a scheme (`pub identity(x) = x` →
`identity<T: Clone>(x: T) -> T` in `crisp.lock`). Drift is E0080.

### 5.3 Closures / Lambdas

```rust
f := |x| x + 1
transform := |data| { cleaned := data.trim(); parse(cleaned) }
list.map(|x| x * 2)
list.fold(0, |acc, x| acc + x)
```

Closures lower to Rust closures. The ownership pass decides `move` vs borrow capture and
emits `move` where required (e.g. captures crossing a `spawn`, §11). `reveal ownership`
shows capture modes.

A local `|x| x` follows the same scheme rule as a named item: it generalizes only
if the closure **escapes** (is returned or stored where callers can instantiate it);
otherwise it is monomorphized at its use sites. First-class closure emit is [#72](https://github.com/jose-compu/crisp/issues/72).

### 5.4 Methods

```rust
type Vec2 = { x: float, y: float }

impl Vec2 = {
    new(x, y)        = Vec2 { x, y }
    magnitude(self)  = sqrt(self.x**2 + self.y**2)
    normalize(self)  = { m := self.magnitude(); Vec2 { x: self.x/m, y: self.y/m } }
    scale(self, f)   = Vec2 { x: self.x*f, y: self.y*f }
}

v := Vec2.new(3.0, 4.0)
m := v.magnitude()      -- 5.0
```

`new` has no `self` → associated function (`Vec2::new`). `self`-taking methods lower
with the receiver mode chosen by §7.

### 5.5 Pipe Operator

```rust
result := data |> parse |> validate |> transform |> serialize
```

Lowers to nested calls: `serialize(transform(validate(parse(data))))`. If a piped
function is a method, the pipe lowers to method-call form. The ownership pass treats the
pipe exactly as the desugared call chain.

---

## 6. Control Flow

### 6.1 If Expressions

```rust
max := if a > b then a else b
result := if cond { compute_a() } else { compute_b() }
```

`if` is an expression → Rust `if`/`else` as an expression. The `then` keyword form is
sugar for the brace form; both lower identically. An `if` used as an expression must
have an `else` (Rust requires both arms), enforced at the Crisp level.

### 6.2 Match Expressions

```rust
describe(color) = match color {
    Color.Red             -> "red"
    Color.Custom(r, g, b) -> "rgb({r}, {g}, {b})"
    _                     -> "other"
}
```

Lowers to Rust `match`. Exhaustiveness is checked by Crisp for early errors and again by
`rustc` (authoritative).

### 6.3 Loops

```rust
for item in collection { process(item) }
for (i, item) in collection.enumerate() { log("{i}: {item}") }
while cond { step() }
loop { if done() then break; tick() }
found := loop { x := next(); if x.matches(q) then break x }
```

`for` lowers over `IntoIterator`. `loop` with `break value` lowers to Rust's
value-producing `loop`. The ownership pass decides whether `for` iterates by value,
`&`, or `&mut` based on body usage — and emits `.iter()` / `.iter_mut()` /
`.into_iter()` accordingly.

### 6.4 Early Return

```rust
find(list, pred) = {
    for item in list { if pred(item) then return some(item) }
    none
}
```

---

## 7. Ownership and Borrowing

### 7.1 Overview and Status

Crisp infers ownership **globally** and emits explicit Rust ownership annotations. Per
§0.2 and §0.4:

- Inference is **whole-program**: a parameter's mode (`&`, `&mut`, owned) can depend on
  how callers use the returned value and on what the callee does. This is the v0.1.0
  ambition, retained.
- Crisp's analysis is **not the soundness boundary**. It exists to (a) decide what Rust
  to emit and (b) produce Crisp-level diagnostics. The emitted Rust is then checked by
  `rustc`, which is authoritative. If `rustc` rejects emitted code, that is a Crisp
  compiler bug (§17.3), surfaced against the originating Crisp span.

This resolves the v0.1.0 Rule 2 / Rule 4 conflict (read-vs-return) honestly: Crisp picks
a mode, emits it, and if the choice doesn't borrow-check, the transpiler retries with the
next candidate mode before concluding it has a bug (§7.6).

### 7.2 The Inference Pipeline

Run after type inference, over the fully-typed program:

1. **Usage collection.** For every binding and parameter, collect the set of uses:
   `read`, `mutate`, `move-out` (returned by value, stored in a longer-lived structure,
   or passed to a consuming position), and `copy` (if the type is `Copy`).
2. **Constraint generation.** Each use emits a mode constraint. `read` ⇒ at least `&`.
   `mutate` ⇒ at least `&mut`. `move-out` ⇒ owned. Constraints combine by a lattice:
   `&` ⊑ `&mut` ⊑ owned.
3. **Global solve.** Constraints propagate across call edges (a callee that moves a
   parameter forces callers to provide ownership). Fixpoint over the call graph.
4. **Region/lifetime assignment.** Assign lifetimes; rely on Rust elision where it
   applies, emit explicit `'a` where multiple input references compete for an output
   (§8).
5. **Emission.** Lower each parameter/binding to the solved mode; insert `.clone()`,
   `&`, `&mut`, or `move` as needed.

### 7.3 Inference Rules (refined)

The v0.1.0 rules are retained but reordered to be a **lattice join**, not a
priority list — this is what removes the Rule 2/4 contradiction:

- **Copy types** (`int`, `float`, `bool`, `char`, and `: Copy` structs) are always
  copied. They never force ownership on a parameter.
- A binding's mode is the **join of all its uses**. A value that is both read and
  moved-out resolves to *owned* (owned ⊒ read), not to a conflict.
- A parameter only read across the whole program ⇒ emitted `&`.
- A parameter mutated ⇒ emitted `&mut`.
- A parameter moved-out anywhere ⇒ emitted owned; at call sites that only read it
  afterward, the transpiler inserts `.clone()` **only if** the value is used again after
  the move; otherwise the move stands.

```rust
greet(name) = "hello " ++ name
-- only read -> greet(name: &str) -> String

push_value(data, v) = data.push(v)
-- data mutated -> push_value(data: &mut Vec<T>, v: T)

into_boxed(value) = Box.new(value)
-- moved-out -> into_boxed(value: T) -> Box<T>
```

### 7.4 Clone Insertion Policy (new)

Because the target is Rust and `.clone()` is the standard escape from a borrow conflict,
Crisp will **insert clones automatically** in one narrowly-defined case: a `Copy`-like
ergonomic situation where a value is moved and then read, *and* the type implements
`Clone`. This insertion is:

- **Off by default** for non-`Copy` heap types (cloning a `Vec` silently is a footgun).
- **Reported always** by `reveal ownership` as `[auto-clone @ line N]`.
- **Suppressible** with an explicit `own`/`&` annotation, which the programmer uses to
  say "I meant to move/borrow, error if you can't."

This is a deliberate divergence from "never insert anything": silent `Rc` is forbidden,
but a *reported* clone of a `Clone` type is permitted because it is observable in
`reveal` and cannot cause unsoundness (only performance surprises).

### 7.5 Explicit Annotations

```rust
read_only(data: &Vec<int>)      = data.len()
modify(data: &mut Vec<int>)     = data.push(0)
consume(data: own Vec<int>)     = drop(data)
```

Explicit annotations are **hard constraints**: if the body's usage contradicts them, it
is a Crisp error (not an auto-clone, not a silent widen).

### 7.6 When Inference and rustc Disagree

If the emitted Rust fails to borrow-check:

1. The transpiler consults a small set of **fallback rewrites** (re-borrow, clone a
   `Clone` value, restructure a temporary's lifetime).
2. If a fallback compiles, it is applied and noted in `reveal ownership`.
3. If none compiles, the transpiler emits a **Crisp compiler diagnostic** ("internal:
   could not produce borrow-checking Rust for `f`; please file a bug") mapped to the
   Crisp span, and fails the build.

The user is never shown a raw `rustc` borrow error against generated code (§17.3).

### 7.7 Copy Semantics

```rust
type Point = { x: float, y: float } : Copy
a := Point { x: 1.0, y: 2.0 }
b := a        -- copy; both valid
```

`: Copy` lowers to `#[derive(Copy, Clone)]`. Types containing heap data cannot be
`Copy`; declaring it is a Crisp error mirroring Rust's.

### 7.8 Borrowing Rules

The Rust aliasing invariants apply and are **enforced by rustc**. Crisp's pre-check is a
best-effort early-warning, explicitly not a guarantee (§0.4).

---

## 8. Lifetimes and Regions

### 8.1 Strategy

Crisp emits lifetimes only where Rust cannot elide them. The pass:

- Leaves single-input-reference functions to Rust elision (no annotation emitted).
- Emits explicit `'a` when multiple input references could be the source of an output
  reference, or when a struct stores a reference.

### 8.2 Inferred, Common Case

```rust
first(data) = data[0]
-- emitted: fn first<T>(data: &[T]) -> &T   (elided lifetime, rustc infers)
```

### 8.3 Explicit, Rare Case

The v0.1.0 tick syntax is retained but its meaning is pinned to Rust lifetimes:

```rust
longest('a: x, 'a: y) = if x.len() > y.len() then x else y
-- emitted: fn longest<'a>(x: &'a str, y: &'a str) -> &'a str
```

`'name: param` declares that `param` lives in region `name`; shared names unify to one
Rust lifetime parameter.

### 8.4 Structs Holding References

```rust
type Slice = { data: &[int] }
-- emitted: struct Slice<'a> { data: &'a [i64] }
```

Lifetime parameters on types are inferred and emitted; `reveal lifetimes` shows them.

### 8.5 Deallocation

There is **no separate deallocation guarantee in Crisp** beyond what Rust provides.
Drop timing, drop order (reverse declaration), and determinism are **Rust's `Drop`
semantics**, inherited wholesale. The v0.1.0 "deterministic deallocation guarantee" is
restated as: *Crisp introduces no GC and no hidden reference counting; all drop behavior
is exactly Rust's.*

### 8.6 Explicit Reference Counting

```rust
node       := rc(TreeNode { value: 1, children: [] })   -- Rc<TreeNode>
shared_ref := node.clone()                                -- Rc::clone
counter    := arc(Mutex.new(0))                           -- Arc<Mutex<i64>>
```

`rc`/`arc` lower to `Rc::new`/`Arc::new`. Never inserted by the compiler.

---

## 9. Error Handling

### 9.1 Model: Ambient Propagation, Uniform Lowering

Functions do not declare error types. The compiler infers fallibility from the body. The
key v0.2.0 decision (§0.3): **all fallible functions lower to
`Result<T, CrispError>`**, where `CrispError` is a single program-global enum generated
by the transpiler, with one variant per distinct error type produced anywhere in the
program.

```rust
read_config(path) = {
    text   := fs.read(path)     -- IoError
    config := parse(text)       -- ParseError
    config.validate()           -- ValidationError
    config
}
-- reveal errors:
--   read_config(path: &str) -> Config ! IoError | ParseError | ValidationError
-- emitted Rust:
--   fn read_config(path: &str) -> Result<Config, CrispError> {
--       let text = fs::read(path)?;          // From<IoError> for CrispError
--       let config = parse(&text)?;          // From<ParseError>
--       config.validate()?;                  // From<ValidationError>
--       Ok(config)
--   }
```

The `!` set in `reveal` is **documentation computed by the analysis**, not a Rust type.
Every fallible function has the *same* Rust return type modulo `T`. Propagation lowers to
the `?` operator; the generated `From<E> for CrispError` impls make `?` type-check.

### 9.2 The Generated `CrispError` Enum

```rust
// generated, program-global
enum CrispError {
    Io(std::io::Error),
    Parse(ParseError),
    Validation(ValidationError),
    // ...one variant per error type reaching any ? in the program
    Thrown(String),          // for `throw` of ad-hoc messages
}
```

**Tradeoff (stated explicitly).** This erases per-function error precision from the
*type system* — every caller sees `CrispError`, not a function-specific set. The
precision is recovered by `reveal errors`. The alternative, `Box<dyn Error>`, is
available via a manifest flag (`error_model = "boxed"`) for programs that prefer dynamic
errors over a global enum; the enum is the default because it allows exhaustive `catch`
(§9.3) to be checked.

### 9.3 Handling with `catch`

```rust
main() = {
    cfg := read_config("app.toml") catch err -> {
        log("config error: {err}")
        Config {}
    }
    run(cfg)
}
```

`catch err -> recovery` lowers to a `match` on the `Result`:

```rust
let cfg = match read_config("app.toml") {
    Ok(v) => v,
    Err(err) => { log(format!("config error: {err}")); Config::default() }
};
```

The recovery block must produce the success type (checked at Crisp level and by `rustc`).

### 9.4 Selective Catching

```rust
cfg := read_config("app.toml")
    catch IoError    -> Config {}
    catch ParseError(e) -> panic("bad config: {e}")
```

Lowers to a `match` on `CrispError` variants. Because `CrispError` is a closed
(program-global) enum, Crisp can tell whether the listed `catch` arms cover the
function's inferred error set and warn on unreachable or missing arms — a capability the
`Box<dyn Error>` model could not offer.

### 9.5 Explicit Error Declaration

```rust
read_config(path) -> Config ! IoError | ParseError = { ... }
```

Declaring the set is a **constraint**: if the body can produce an error outside the set,
Crisp errors. The Rust return type is still `Result<Config, CrispError>`; the declared
set narrows what `catch`-checking and `reveal` consider reachable, and inserts a
debug-assert that no other variant escapes (the assert is compiled out in release).

### 9.6 `throw`

```rust
validate(config) = {
    if config.port == 0 then throw ValidationError("port cannot be 0")
    config
}
```

`throw E` lowers to `return Err(CrispError::from(E))`. `throw "msg"` of a bare string
lowers to the `Thrown(String)` variant.

### 9.7 Explicit `?` (new)

For interop with Rust APIs that return `Result<T, E>` directly, the postfix `?` operator
is available and lowers to Rust `?`:

```rust
parse_num(s) = s.trim().parse::<int>()?      -- explicit try on a Rust Result
```

This makes the boundary between ambient and explicit error handling visible when calling
into hand-written Rust (§14).

### 9.8 Non-Fallible Assertion

```rust
add(x, y) -> int !never = x + y
```

`!never` (note: spelled with the renamed bottom concept but kept as the established
annotation) asserts the function makes no fallible call. If the body contains one, Crisp
errors. Non-fallible functions lower to a plain `-> T` Rust signature (no `Result`
wrapper), which is what keeps hot paths free of error plumbing.

### 9.9 Panic

```rust
critical_init() = {
    db := connect_db() catch _ -> panic("cannot start without database")
    db
}
```

`panic` lowers to Rust `panic!`. Not catchable, not part of the error type system —
identical to Rust semantics.

---

## 10. Pattern Matching

Unchanged in semantics from v0.1.0; all forms lower to Rust patterns and are
exhaustiveness-checked twice (Crisp then `rustc`).

```rust
eval(expr) = match expr {
    Expr.Literal(n) -> n
    Expr.Add(a, b)  -> eval(a) + eval(b)
    Expr.Neg(inner) -> -eval(inner)
}
```

Supported pattern forms (literal, tuple, struct, nested, or-patterns, guards, `@`
bindings, slice patterns) map one-to-one onto Rust. `if-let` lowers to Rust `if let`:

```rust
if some(value) := maybe { process(value) }
```

> **Lowering note (new).** The `@` rest-binding form
> `[first, second, ..rest @ _]` lowers to Rust `[first, second, rest @ ..]`. Crisp
> normalizes the v0.1.0 ordering to Rust's required position.

---

## 11. Concurrency

### 11.1 Async / Await

```rust
fetch_data(url) = async {
    response := http.get(url).await
    body     := response.text().await
    parse(body)
}
```

Lowers to `async fn` / `async` blocks and `.await`. Crisp does not bundle a runtime; the
manifest selects one (`runtime = "tokio"` by default), and the transpiler emits the
matching attributes (`#[tokio::main]` on an async `main`).

### 11.2 Spawn

```rust
main() = async {
    t1 := spawn fetch_data("https://a.com")
    t2 := spawn fetch_data("https://b.com")
    (r1, r2) := await_all(t1, t2)
    merge(r1, r2)
}
```

`spawn` lowers to the runtime's spawn (`tokio::spawn`). Values captured by a spawned task
are forced to `move` capture by the ownership pass, and non-`Send` captures are a Crisp
error mirroring Rust's `Send` requirement.

### 11.3 Ownership Across Tasks

- Values used in a `spawn` body are `move`-captured (emitted `move` closures).
- Shared access requires explicit `arc(...)` / `shared(...)`; the compiler never inserts
  it.
- Data races are rejected by `rustc` via `Send`/`Sync`; Crisp pre-checks for diagnostics.

```rust
counter := arc(Mutex.new(0))
tasks := (0..10).map(|_| {
    c := counter.clone()
    spawn async { c.lock().update(|v| v + 1) }
})
await_all(tasks)
```

### 11.4 Channels

```rust
(tx, rx) := channel<int>()
spawn async { for i in 0..10 { tx.send(i).await }; tx.close() }
for value in rx { log("received: {value}") }
```

Lowers to the runtime's channel type.

---

## 12. Module System

### 12.1 File-Based Modules

File structure maps to Rust modules. No `mod` declarations in source; the transpiler
generates the `mod` tree (and `mod.rs`/inline `mod` items) from the directory layout.

```rust
project/
  main.crp
  math/ vector.crp  matrix.crp
  io/   reader.crp
```

### 12.2 Visibility

Private by default; `pub` exports. (The v0.1.0 duplicate `type Vec3` example is removed.)

```rust
-- math/vector.crp
pub type Vec3 = { x: float, y: float, z: float }
pub add(a: Vec3, b: Vec3) = Vec3 { x: a.x+b.x, y: a.y+b.y, z: a.z+b.z }
helper(v) = ...     -- private
```

Lowers to `pub`/private Rust items with `pub(crate)` used for items public within the
sealed crate but not re-exported.

### 12.3 Imports

```rust
use math.vector { Vec3, add }
use math.matrix { Mat4, multiply as mat_mul }
use std.collections { HashMap, HashSet }
```

`.`-paths lower to Rust `::`-paths. `use a.b { * }` lowers to a glob import (discouraged,
linted).

### 12.4 Re-exports

```rust
pub use math.vector { Vec3, add }
```

Lowers to `pub use`.

### 12.5 Sealed Crates and the Signature Lockfile (new — required by §0.2)

Because ownership and error sets are inferred globally, a function's true Rust signature
is not known until whole-program analysis runs. To allow a Crisp crate to be *depended
upon* without forcing every downstream build to re-analyze its internals, Crisp
introduces a **sealed crate boundary**:

- A crate's `pub` API has its fully-resolved signatures (types, ownership modes,
  lifetimes, and the slice of `CrispError` it can produce) **frozen into a lockfile**
  (`crisp.lock`) at publish time.
- Downstream crates analyze against the lockfile signatures, not the source.
- If a crate's `pub` API would resolve to a different signature than the lockfile
  records (e.g. an edit changed an inferred ownership mode), the build fails with a
  "sealed signature drift" error, prompting a version bump.

This is the explicit price of whole-program inference: **inside** a crate, inference is
global and signatures are fluid; **across** crate boundaries, signatures are pinned.
`reveal seal <crate>` prints the frozen API.

---

## 13. Memory Model

### 13.1 Placement

Stack/heap placement is Rust's. Primitives and `Copy` structs are by-value; `Vec`,
`String`, `HashMap`, etc. are heap-backed with stack metadata. There is no `new`/`malloc`.

### 13.2 Drop

Drop timing and order (reverse declaration within a scope) are **Rust's `Drop`**,
inherited verbatim (§8.5). Crisp adds nothing here.

### 13.3 Custom Destructors

```rust
type Connection = { handle: RawHandle, name: str }
impl Drop for Connection = {
    drop(self) = { log("closing {self.name}"); raw_close(self.handle) }
}
```

Lowers to `impl Drop`.

### 13.4 Move Semantics

```rust
a := make_large_struct()
b := a              -- moved; using `a` after is a Crisp error (and a rustc error)
```

### 13.5 Clone

```rust
a := vec![1,2,3]
b := a.clone()       -- explicit deep copy
```

See §7.4 for the narrow case where Crisp may insert a *reported* clone.

### 13.6 Memory Safety Guarantees (reframed per §0.4)

The following hold **of any Crisp program whose generated Rust compiles** — they are
properties of the emitted Rust, verified by `rustc`, not independent claims of Crisp's
own checker:

- No use-after-free, no double-free, no dangling references.
- No data races (`Send`/`Sync` enforced by `rustc`).
- No null dereference (no null; `?T`/`Option`).
- Bounds-checked indexing.
- Drop behavior identical to Rust.

If Crisp's pre-check accepts a program but the generated Rust does **not** compile, that
is a Crisp compiler bug (§17.3), not an unsound program — the program never builds, so no
unsafe binary is produced. This is what "belt-and-suspenders, rustc is truth" means in
practice.

---

## 14. Foreign Function Interface (FFI)

### 14.1 Calling C

```rust
extern "C" {
    malloc(size: uint) -> &mut u8
    free(ptr: &mut u8)
    printf(fmt: &c_str, ..) -> int
}
```

Lowers to a Rust `extern "C"` block. Calls require `unsafe`:

```rust
allocate(n) = unsafe {
    ptr := malloc(n)
    if ptr.is_null() then panic("allocation failed")
    ptr
}
```

### 14.2 Calling Rust (now primary, since the target IS Rust) (changed)

Because Crisp emits Rust, calling existing Rust is **not FFI** — it is direct module use.
A hand-written Rust crate is added as a dependency and imported:

```rust
use rust::serde_json { from_str, to_string }    -- a real Rust crate
data := from_str(text)?                           -- explicit ? on Rust's Result
```

Rust functions returning `Result<T, E>` interoperate via the explicit `?` (§9.7); their
`E` is absorbed into `CrispError` through a generated `From` impl, or surfaced directly
if the programmer binds it.

The v0.1.0 `extern "rust"` block is **removed** as redundant.

### 14.3 Exporting from Crisp

```rust
pub extern "C" crisp_add(a: i32, b: i32) -> i32 = a + b
```

Lowers to `#[no_mangle] pub extern "C" fn`. Exported functions may not be fallible in the
ambient sense (no `Result` across the C boundary); they must be `!never` or return an
explicit error code.

### 14.4 Unsafe

`unsafe` blocks lower to Rust `unsafe`. Required for: foreign calls, raw-pointer deref,
mutable-global access. Unsafe is a code smell; keep it minimal and encapsulated.

---

## 15. Standard Library Overview

The prelude maps onto Rust's `std` plus a thin Crisp shim.

### 15.1 Core Types

```rust
std.option   -- ?T              -> Option<T>
std.result   -- Result<T, E>    -> Result<T, E>   (interop/explicit only; §9.7, §14)
std.string   -- str/&str        -> String / &str
std.vec      -- vec<T>          -> Vec<T>
std.map      -- map<K,V>        -> HashMap / BTreeMap
std.set      -- set<T>          -> HashSet / BTreeSet
```

`Result` is listed once, here, and is for explicit/interop use — the ambient path uses
`CrispError` (§9). This removes the v0.1.0 Option/Result overlap.

### 15.2 IO / Net

```rust
std.io   std.fs   std.net   std.http
```

Map onto `std::io`, `std::fs`, and selected crates (`std.http` → a chosen HTTP crate
named in the manifest).

### 15.3 Concurrency

```rust
std.async  -> runtime (tokio default)
std.sync   -> Mutex, RwLock, Arc, channels
std.atomic -> atomics
```

### 15.4 Traits

```rust
Show Eq Ord Hash Clone Copy Drop Default Into From Iterator
Add Sub Mul Div   -- operator overloading -> std::ops
```

Each maps to the corresponding `std` trait (`Show` → `Display`).

---

## 16. Tooling: `reveal`

`reveal` reconstructs everything the surface syntax omits **and** the precision the
uniform `CrispError` erases.

### 16.1 Commands

| Command | Description |
|---------|-------------|
| `reveal types <file>` | Inferred type signatures (incl. inserted `Box` for recursion) |
| `reveal ownership <file>` | Borrow/move/copy modes, capture modes, auto-clones |
| `reveal lifetimes <file>` | Emitted lifetime parameters |
| `reveal errors <file>` | Per-function reachable `CrispError` variant set (the `!` sets) |
| `reveal traits <file>` | Trait/shape impls, including generated shape traits & wrappers |
| `reveal rust <file>` | **(new)** The emitted Rust for the file |
| `reveal seal <crate>` | **(new)** The frozen sealed-crate API (§12.5) |
| `reveal expand <file>` | Fully-annotated equivalent Crisp source |
| `reveal diff <file>` | Side-by-side: Crisp source vs. emitted Rust |

`reveal map <file>` (renamed from v0.1.0 `reveal memory`) shows alloc/drop points;
because drop is Rust's, it now annotates against the emitted Rust spans.

### 16.2 Example

Source:

```rust
read_config(path) = {
    text   := fs.read(path)
    config := parse(text)
    config.validate()
    config
}
greet(name) = "hello " ++ name
```

`reveal types`:

```rust
read_config(path: &str) -> Config ! IoError | ParseError | ValidationError
greet(name: &str) -> String
```

`reveal rust`:

```rust
fn read_config(path: &str) -> Result<Config, CrispError> {
    let text = std::fs::read_to_string(path)?;
    let config = parse(&text)?;
    config.validate()?;
    Ok(config)
}
fn greet(name: &str) -> String { format!("hello {name}") }
```

### 16.3 LSP

`reveal` powers ghost-text type hints, hover signatures, ownership/lifetime overlays,
the reachable-error-set on each call, and a "show emitted Rust" code lens.

---

## 17. Compiler / Transpiler Architecture

### 17.1 Pipeline (revised: targets Rust via CIR)

```rust
Source (.crp)
   |  Lexer            tokenization
   v
   |  Parser           AST (brace-delimited, expression-based)
   v
   |  Name Resolution  imports, symbol table, sealed-crate lockfile load (§12.5)
   v
   |  Type Inference   HM-style + constraint solving        (§3.4)
   v
   |  Ownership Pass    global usage->mode dataflow fixpoint (§7)
   v
   |  Region Pass       lifetime assignment                 (§8)
   v
   |  Error Pass        fallibility + reachable CrispError sets (§9)
   v
   |  CIR Generation    typed intermediate representation:
   |                    - Box insertion for recursive enums
   |                    - shape-trait + impl synthesis
   |                    - default-field builder synthesis
   |                    - clone/borrow/move materialization
   v
   |  Rust Emission     pretty-printed Rust from CIR
   v
   |  rustc             AUTHORITATIVE compile + borrow-check
   v
 Native Binary
```

CIR is a **typed, ownership-resolved IR**: every node carries its resolved type and
ownership mode, so Rust emission is a near-mechanical pretty-print. Keeping a typed IR
(rather than emitting Rust text directly from the AST) is what makes the shape-trait
synthesis, Box insertion, and clone materialization tractable and inspectable
(`reveal rust` prints from CIR).

### 17.2 Crisp Diagnostics vs. rustc Diagnostics

- **Crisp-level errors** (type mismatch, ambiguous inference, ownership contradiction
  against an explicit annotation, non-exhaustive match, sealed-signature drift) are
  reported against Crisp source with Crisp wording. These are the user's responsibility.
- **rustc errors on generated code** are, by definition (§0.4), **Crisp compiler bugs**.

### 17.3 Handling rustc Errors on Generated Code

When `rustc` rejects emitted Rust:

1. The transpiler attempts the §7.6 fallback rewrites.
2. Failing that, it maps the `rustc` span back to the originating Crisp span via the CIR
   source map and emits:
   `internal compiler error: generated Rust failed to compile at <crisp-span>. This is
   a Crisp compiler bug; please report it. (rustc said: <summary>)`
3. The build fails. No binary is produced. The user never debugs raw generated Rust
   unless they explicitly run `reveal rust`.

### 17.4 Example Crisp-Level Error

```rust
ERROR [E0042]: ownership contradicts annotation in `transfer`
  --> lib.crp:15:5
   |
15 |     process(data)
   |             ^^^^ `data` is annotated `&` but `process` moves it
   |
   = note: either drop the `&` annotation (let inference choose `own`)
           or clone: process(data.clone())
```

---

## 18. Package Management

### 18.1 Structure

```rust
my_project/
    crisp.toml        -- manifest
    crisp.lock        -- resolved deps + sealed signatures (§12.5)
    src/ main.crp lib.crp util/helpers.crp
    tests/ bench/
```

### 18.2 Manifest

```toml
[package]
name = "my_project"
version = "0.1.0"
edition = "2026"

[build]
target = "rust"          # only target in v0.2.0
runtime = "tokio"        # async runtime emitted for async main
error_model = "enum"     # "enum" (default, CrispError) or "boxed" (Box<dyn Error>)

[dependencies]
http = "1.2"
json = { version = "0.5", features = ["pretty"] }
# direct Rust crates are allowed and imported via `use rust::...`
serde_json = { rust = true, version = "1" }
```

### 18.3 Commands

```rust
crisp build      -- analyze -> emit Rust -> invoke rustc
crisp run        -- build + run
crisp test       -- run tests
crisp check      -- analyze + emit; typecheck emitted Rust via cargo check (fast)
crisp emit       -- (new) emit Rust to target/rust/ and stop
reveal expand src/-- annotated Crisp for the project
reveal rust src/  -- (new) emitted Rust for the project
```

`crisp` invokes the system `rustc`/`cargo`; the emitted crate is an ordinary Cargo
project under `target/rust/`, so the generated output is itself buildable and auditable.

---

## 19. Testing

```rust
test "addition works" = { assert_eq(add(2, 3), 5) }

test "vector magnitude" = {
    v := Vec2.new(3.0, 4.0)
    assert_approx(v.magnitude(), 5.0, epsilon: 0.001)
}

test "parse failure" = {
    result := try_parse("invalid")           -- returns ?T or Result for testing
    assert_match(result, none)
}
```

`test` blocks lower to `#[test] fn` (and `#[tokio::test]` for async tests). Compile-fail
expectations (the v0.1.0 "ownership transfer" example, which asserted a *non*-compiling
program) move to a dedicated form, since a `#[test]` cannot contain code that fails to
compile:

```rust
test_compile_fail "use after move" = {
    data := vec![1,2,3]
    consume(data)
    use_again(data)     -- expected: Crisp E-move error
}
```

This lowers to a `trybuild`-style harness rather than a runtime test. (v0.1.0 conflated
compile-time and run-time tests in one `test` form; this separates them.)

---

## 20. Complete Example

```rust
-- src/main.crp
use std.fs
use std.http { Server, Request, Response }
use std.json

type Config = {
    host: str  = "localhost"
    port: uint = 8080
    max_connections: uint = 100
}

type AppState = {
    config: Config
    request_count: arc(Atomic<uint>)
}

read_config(path) = {
    text := fs.read(path) catch _ -> return Config {}
    json.decode(text) catch err -> {
        log("config parse error: {err}, using defaults")
        Config {}
    }
}

handle_request(state, req) = {
    state.request_count.fetch_add(1)
    match req.path {
        "/"       -> Response.ok("welcome to crisp server")
        "/stats"  -> {
            count := state.request_count.load()
            Response.ok(json.encode({ requests: count }))
        }
        "/health" -> Response.ok("ok")
        _         -> Response.not_found("404: {req.path}")
    }
}

pub main() = async {
    config := read_config("config.json")
    state  := AppState { config: config, request_count: arc(Atomic.new(0)) }
    server := Server.bind("{config.host}:{config.port}").await
    log("listening on {config.host}:{config.port}")
    server.serve(|req| handle_request(state, req)).await
}
```

`reveal rust src/main.crp` emits a complete Cargo crate: `Config` with a generated
builder for defaults, `AppState` with `Arc<AtomicU64>`, `read_config` returning
`Result<Config, CrispError>` with `?`/`match` lowering for the `catch` arms,
`handle_request` with `state: &AppState` (inferred shared borrow) and `req` by value, and
a `#[tokio::main] async fn main`. The `move` capture on the `|req| ...` closure is
inserted because it crosses into `serve`; `reveal ownership` reports it.

---

## Appendix A: Grammar (EBNF, v0.2.0 deltas)

Changes from v0.1.0 grammar:

```ebnf
(* bitwise operators respelled to avoid collision *)
bit_and = "&&&" ; bit_or = "|||" ; bit_xor = "^^^" ;

(* explicit try operator added *)
try_expr = expr "?" ;

(* error type annotation unchanged in shape, semantics uniform-lowered *)
error_type = "!" type { "|" type } | "!never" ;

(* compile-fail test form added *)
test_item = "test" string_lit "=" block
          | "test_compile_fail" string_lit "=" block ;

(* extern "rust" removed; extern only takes "C" now *)
extern_block = "extern" "\"C\"" "{" { extern_fn } "}" ;

(* bottom type renamed *)
(* primitive: Never  (was: never) *)
```

The remainder of the grammar is as in v0.1.0 Appendix A.

---

## Appendix B: Comparison with Rust (v0.2.0)

| Feature | Rust | Crisp v0.2.0 |
|---------|------|--------------|
| Output | native via LLVM | **Rust source**, then `rustc` |
| Type annotations | required on signatures | inferred, optional |
| Lifetimes | required when ambiguous | inferred; emitted where Rust needs them |
| Ownership | explicit | inferred globally, **emitted explicitly** |
| Error handling | `Result<T,E>` + `?` | ambient -> uniform `Result<T, CrispError>` |
| Per-fn error precision | in the type | in `reveal`, not the type |
| Separate compilation | stable ABI | **none**; sealed crate + lockfile (§12.5) |
| Soundness | rustc | **rustc** (Crisp defers; §0.4) |
| Shapes | (none) | structural; lowered to generated traits |
| `?T` vs Result | `Option` / `Result` | `?T` ambient-only split (§9, §15) |
| Backend | LLVM | rustc (which uses LLVM) |

---

## Appendix C: Migration from v0.1.0

| v0.1.0 | v0.2.0 | Action |
|--------|--------|--------|
| `never` (bottom type) | `Never` | Rename in source |
| bitwise `& \| ^` | `&&& \|\|\| ^^^` | Respell bitwise ops |
| `extern "rust" { ... }` | direct `use rust::crate` | Remove extern-rust blocks; add dep |
| direct LLVM target | `target = "rust"` | Set in manifest (only option) |
| implicit per-fn error types | uniform `CrispError` | No source change; `reveal errors` for precision |
| `reveal memory` | `reveal map` | Rename invocation |
| `test` containing non-compiling code | `test_compile_fail` | Move compile-fail cases |
| duplicate `type Vec3` (§12.2) | single `pub type Vec3` | Delete the dup |
| "deterministic dealloc guarantee" | "Rust Drop semantics inherited" | Conceptual only |
| whole-program assumed free | sealed crates + `crisp.lock` | Publish-time signature freeze |

---

*This specification is a living document. Version 0.2.0-draft.*