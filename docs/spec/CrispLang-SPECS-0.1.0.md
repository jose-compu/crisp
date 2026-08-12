# The Crisp Programming Language Specification

**Version:** 0.1.0-draft  
**Status:** Abstract Design Specification  
**File Extension:** `.crp`  
**Compiler:** `crispc`  
**Toolchain:** `reveal`

---

## 1. Philosophy

Crisp is a systems programming language that provides Rust-level memory safety and performance guarantees with significantly reduced syntactic verbosity. The core design principle is:

> **Explicit on demand, implicit by default.**

The source code is optimized for writing and local readability. The compiler performs aggressive inference across types, ownership, lifetimes, and error propagation. A companion toolchain (`reveal`) surfaces all implicit information on demand — in the terminal, in LSP, or as fully-annotated source output.

### 1.1 Design Goals

- Memory safety without garbage collection, verified at compile time.
- Type safety through aggressive inference — annotations are optional unless genuinely ambiguous.
- Ownership and borrowing inferred from usage patterns, not declared.
- Lifetimes resolved by the compiler through region analysis, almost never written.
- Error propagation is implicit by default, explicit when desired.
- Compact syntax that eliminates ceremonial keywords and punctuation.
- Full interoperability with Rust and C via shared LLVM backend.
- Tooling-first philosophy: the `reveal` toolchain makes all implicit information visible.

### 1.2 Non-Goals

- Crisp is not a scripting language. It compiles to native code.
- Crisp does not use garbage collection or runtime reference counting unless explicitly requested.
- Crisp does not sacrifice compile-time safety for syntactic convenience.

---

## 2. Lexical Structure

### 2.1 Source Encoding

All Crisp source files are UTF-8 encoded.

### 2.2 Comments

```rust
-- This is a single-line comment

{- This is a
   block comment -}

{- Block comments {- can nest -} -}
```

### 2.3 Identifiers

Identifiers consist of letters, digits, and underscores. They must begin with a letter or underscore.

```rust
valid_name
_private
Point3D
```

Identifiers beginning with `_` suppress unused-variable warnings.

### 2.4 Keywords

```rust
if then else match with for in while do
return break continue
type trait shape impl
own mut ref shared rc
async await spawn
use as mod
true false none some
catch throw
pub
```

### 2.5 Operators

```rust
Arithmetic:    +  -  *  /  %  **
Comparison:    == != < > <= >=
Logical:       && || !
Bitwise:       & | ^ ~ << >>
Assignment:    = += -= *= /= %=
Binding:       :=  mut:=
Range:         ..  ..=
Concatenation: ++
Pipe:          |>
Constraint:    +  (in type position, unions constraints)
Access:        .
```

### 2.6 Delimiters

```rust
{ }    -- blocks, struct literals, trait bodies
( )    -- grouping, tuples, function arguments
[ ]    -- array/slice literals, indexing
< >    -- explicit generic parameters (rare)
```

### 2.7 Literals

```rust
-- Integers
42
1_000_000
0xFF
0b1010
0o77

-- Floats
3.14
1.0e-10
1_000.5

-- Strings
"hello world"
"interpolation: {name} is {age} years old"
r"raw string: no \escaping"
"""
  multi-line string
  with {interpolation}
"""

-- Characters
'a'
'\n'
'\u{1F600}'

-- Booleans
true
false

-- Unit
()
```

---

## 3. Type System

### 3.1 Primitive Types

| Type | Description |
|------|-------------|
| `int` | Platform-sized signed integer (i64 on 64-bit) |
| `uint` | Platform-sized unsigned integer |
| `i8, i16, i32, i64, i128` | Fixed-width signed integers |
| `u8, u16, u32, u64, u128` | Fixed-width unsigned integers |
| `float` | 64-bit floating point (f64) |
| `f32` | 32-bit floating point |
| `bool` | Boolean |
| `char` | Unicode scalar value |
| `str` | UTF-8 string (owned) |
| `()` | Unit type |
| `never` | Bottom type (function never returns) |

### 3.2 Compound Types

```rust
-- Tuples
pair: (int, str) = (42, "hello")

-- Arrays (fixed size)
arr: [int; 5] = [1, 2, 3, 4, 5]

-- Slices (dynamically sized view)
slice: [int] = arr[1..4]

-- Vectors (growable)
vec := vec![1, 2, 3]

-- Maps
map := map!{ "key" -> "value", "a" -> "b" }

-- Option (built-in)
x: ?int = some(42)
y: ?int = none

-- Function types
f: (int, int) -> int
```

### 3.3 User-Defined Types

#### 3.3.1 Structs

```rust
type Point = { x: float, y: float }

-- Construction
p := Point { x: 1.0, y: 2.0 }

-- With defaults
type Config = {
    host: str = "localhost"
    port: uint = 8080
    debug: bool = false
}

cfg := Config { debug: true }   -- host and port use defaults
```

#### 3.3.2 Enums

```rust
type Color =
    | Red
    | Green
    | Blue
    | Custom(u8, u8, u8)

type Result<T, E> =
    | Ok(T)
    | Err(E)

type List<T> =
    | Nil
    | Cons(T, List<T>)
```

#### 3.3.3 Type Aliases

```rust
type Name = str
type Callback = (int) -> bool
type Matrix = vec<vec<float>>
```

### 3.4 Type Inference

Crisp performs full Hindley-Milner type inference extended with ownership and region analysis. Types are inferred for:

- Local bindings (always)
- Function parameters (from usage in body and call sites)
- Function return types (from body)
- Generic type parameters (from usage)
- Trait bounds (from operations used in body)

```rust
-- All types inferred
add(x, y) = x + y

-- reveal shows:
-- add<T: Add>(x: T, y: T) -> T
```

Explicit annotations are accepted anywhere and act as constraints:

```rust
-- Partially annotated
add(x: int, y) = x + y

-- Fully annotated (optional)
add(x: int, y: int) -> int = x + y
```

The compiler requests explicit annotation only when inference is genuinely ambiguous — when multiple valid typings exist and the choice affects semantics.

### 3.5 Shapes (Structural Types)

Shapes define structural constraints that are automatically satisfied by any type with matching structure. They are used for ad-hoc polymorphism in function arguments.

```rust
shape HasPosition = { x: float, y: float }
shape HasName = { name: str }

-- Any type with x: float and y: float satisfies HasPosition
distance(a: HasPosition, b: HasPosition) -> float = {
    dx := a.x - b.x
    dy := a.y - b.y
    sqrt(dx * dx + dy * dy)
}
```

Shapes can also require methods:

```rust
shape Measurable = {
    len(self) -> uint
}
```

Shapes are purely structural. No opt-in or declaration is needed. If the structure matches, the type satisfies the shape.

Anonymous structural types are also valid:

```rust
distance(a: { x: float, y: float }, b: { x: float, y: float }) = ...
```

### 3.6 Traits (Nominal Types)

Traits define semantic contracts. A type must explicitly declare that it implements a trait, because structural match alone does not guarantee semantic correctness.

```rust
trait Serializable = {
    serialize(self) -> bytes
    deserialize(bytes) -> self
}

trait Show = {
    show(self) -> str
}

-- Traits can have default implementations
trait Comparable = {
    compare(self, other: self) -> Ordering

    less_than(self, other: self) -> bool =
        self.compare(other) == Ordering.Less
}
```

#### 3.6.1 Trait Implementation

Full implementation:

```rust
type Point = { x: float, y: float }

impl Show for Point = {
    show(self) = "({self.x}, {self.y})"
}
```

Shorthand implementation — when the type already has all required methods defined as free functions, a body-less `impl` opts the type in:

```rust
-- Free functions already exist
serialize(self: Point) -> bytes = encode_json(self)
deserialize(b: bytes) -> Point = decode_json(b)

-- One-liner opt-in. Compiler verifies signatures match.
impl Serializable for Point
```

If the methods don't match, the compiler reports exactly what's missing or mismatched.

#### 3.6.2 Trait Satisfaction Rules

- A type satisfies a trait ONLY through explicit `impl`.
- Having matching methods is necessary but not sufficient.
- The compiler will suggest `impl` when it detects a structural match.

#### 3.6.3 Shape vs. Trait Summary

| Property | Shape | Trait |
|----------|-------|-------|
| Matching | Structural (automatic) | Nominal (explicit `impl`) |
| Purpose | Ad-hoc polymorphism, convenience | Semantic contracts, guarantees |
| Default methods | No | Yes |
| Function argument use | Yes | Yes |
| Opt-in required | No | Yes |

### 3.7 Combined Constraints

Structural and nominal constraints can be composed with `+`:

```rust
save_named(item: { name: str } + Serializable) = {
    log("saving {item.name}")
    fs.write("{item.name}.bin", item.serialize())
}
```

### 3.8 Generics

Generic type parameters are inferred from usage. Explicit parameters use `<>` syntax when needed:

```rust
-- Inferred generics
identity(x) = x
-- reveal: identity<T>(x: T) -> T

first(pair) = pair.0
-- reveal: first<A, B>(pair: (A, B)) -> A

-- Explicit when needed
convert<T, U>(x: T) -> U where T: Into<U> = x.into()
```

---

## 4. Bindings and Mutability

### 4.1 Immutable Bindings

```rust
x := 42
name := "crisp"
point := Point { x: 1.0, y: 2.0 }
```

Immutable bindings cannot be reassigned. The bound value cannot be mutated.

### 4.2 Mutable Bindings

```rust
counter mut:= 0
counter = counter + 1

list mut:= vec![1, 2, 3]
list.push(4)
```

`mut:=` creates a mutable binding. `=` reassigns. Attempting to reassign an immutable binding is a compile error.

### 4.3 Destructuring

```rust
(a, b) := (1, 2)
{ x, y } := Point { x: 1.0, y: 2.0 }
{ name, ..rest } := config

[first, second, ..tail] := vec![1, 2, 3, 4, 5]
```

### 4.4 Constants

Top-level constants use `=` and must have a value known at compile time:

```rust
MAX_SIZE = 1024
PI = 3.14159265358979
APP_NAME = "crisp-app"
```

---

## 5. Functions

### 5.1 Definition

Functions are defined with `name(params) = body`:

```rust
-- Single expression
add(x, y) = x + y

-- Multi-expression block
sort(list, cmp) = {
    pivot := list.head
    less := list.tail.filter(|x| cmp(x, pivot))
    more := list.tail.filter(|x| !cmp(x, pivot))
    sort(less, cmp) ++ [pivot] ++ sort(more, cmp)
}
```

The last expression in a block is the return value. No `return` keyword needed for the final expression. `return` is available for early exit.

### 5.2 Visibility

Functions are private by default. Prefix with `pub` to export:

```rust
pub add(x, y) = x + y        -- public
helper(x) = x * 2             -- private to module
```

### 5.3 Closures / Lambdas

```rust
-- Closure syntax
f := |x| x + 1
g := |x, y| x * y

-- Multi-line closure
transform := |data| {
    cleaned := data.trim()
    parsed := parse(cleaned)
    validate(parsed)
}

-- As arguments
list.map(|x| x * 2)
list.filter(|x| x > 0)
list.fold(0, |acc, x| acc + x)
```

### 5.4 Methods

Methods are functions defined within a type's namespace using `impl` blocks:

```rust
type Vec2 = { x: float, y: float }

impl Vec2 = {
    new(x, y) = Vec2 { x, y }

    magnitude(self) = sqrt(self.x ** 2 + self.y ** 2)

    normalize(self) = {
        m := self.magnitude()
        Vec2 { x: self.x / m, y: self.y / m }
    }

    scale(self, factor) = Vec2 { x: self.x * factor, y: self.y * factor }
}

v := Vec2.new(3.0, 4.0)
m := v.magnitude()       -- 5.0
```

### 5.5 Pipe Operator

The pipe operator passes the left-hand value as the first argument to the right-hand function:

```rust
result := data
    |> parse
    |> validate
    |> transform
    |> serialize

-- Equivalent to:
result := serialize(transform(validate(parse(data))))
```

---

## 6. Control Flow

### 6.1 If Expressions

`if` is an expression and always returns a value:

```rust
max := if a > b then a else b

message := if status == 200 then "ok"
           else if status == 404 then "not found"
           else "error: {status}"
```

Block form:

```rust
result := if condition {
    compute_a()
    finalize_a()
} else {
    compute_b()
    finalize_b()
}
```

### 6.2 Match Expressions

Pattern matching with exhaustiveness checking:

```rust
describe(color) = match color {
    Color.Red -> "red"
    Color.Green -> "green"
    Color.Blue -> "blue"
    Color.Custom(r, g, b) -> "rgb({r}, {g}, {b})"
}
```

Guards:

```rust
classify(n) = match n {
    0 -> "zero"
    x if x > 0 -> "positive"
    _ -> "negative"
}
```

Destructuring in match:

```rust
process(msg) = match msg {
    { kind: "text", body } -> handle_text(body)
    { kind: "image", url, .. } -> handle_image(url)
    _ -> handle_unknown(msg)
}
```

### 6.3 Loops

```rust
-- For loop (iterates over anything iterable)
for item in collection {
    process(item)
}

-- With index
for (i, item) in collection.enumerate() {
    log("{i}: {item}")
}

-- While loop
while condition {
    step()
}

-- Infinite loop
loop {
    if done() then break
    tick()
}

-- Loop expressions return values via break
found := loop {
    item := next()
    if item.matches(query) then break item
}
```

### 6.4 Early Return

```rust
find(list, pred) = {
    for item in list {
        if pred(item) then return some(item)
    }
    none
}
```

---

## 7. Ownership and Borrowing

### 7.1 Overview

Crisp provides Rust-equivalent ownership guarantees — single ownership, no data races, no use-after-free, no double-free — but infers ownership semantics from usage rather than requiring annotations.

The compiler runs a three-phase analysis on every function:

1. **Usage analysis** — scans how each value is used: read, mutated, moved, returned, stored.
2. **Ownership assignment** — assigns move, borrow, or copy semantics based on usage.
3. **Lifetime resolution** — computes scoped regions from data flow.

### 7.2 Inference Rules

The compiler applies the following rules in order:

**Rule 1: Copy types are always copied.**  
Primitives (`int`, `float`, `bool`, `char`) and small value types are implicitly `Copy`. They are always copied on assignment or function call.

**Rule 2: If a value is only read, it is auto-borrowed.**

```rust
greet(name) = "hello " ++ name
-- Inferred: greet(name: &str) -> str
-- `name` is only read, so it's borrowed.
```

**Rule 3: If a value is mutated, it is auto-mut-borrowed.**

```rust
push_value(data, v) = data.push(v)
-- Inferred: push_value(data: &mut Vec<T>, v: T)
-- `data` is mutated, so it's mutably borrowed.
```

**Rule 4: If a value is consumed (moved into another structure, returned by value, or passed to a consuming function), it is auto-moved.**

```rust
into_boxed(value) = Box.new(value)
-- Inferred: into_boxed(value: own T) -> Box<T>
-- `value` is consumed by Box.new, so it's moved.
```

**Rule 5: If the compiler cannot determine a unique ownership strategy, it reports an error and requests annotation.**

```rust
-- ERROR: `x` could be borrowed or moved here. Annotate with `&x` or `own x`.
ambiguous(x) = {
    store(x)     -- consumes x?
    read(x)      -- reads x?
}
```

### 7.3 Explicit Annotations

When needed, ownership can be stated explicitly:

```rust
-- Explicit borrow
read_only(data: &Vec<int>) = data.len()

-- Explicit mutable borrow
modify(data: &mut Vec<int>) = data.push(0)

-- Explicit move (take ownership)
consume(data: own Vec<int>) = drop(data)
```

### 7.4 Copy Semantics

Types can be marked as `Copy` to always be copied rather than moved:

```rust
type Point = { x: float, y: float } : Copy

-- Now Point is always copied on assignment
a := Point { x: 1.0, y: 2.0 }
b := a        -- copy, both a and b are valid
```

Types containing only `Copy` fields can derive `Copy`. Types containing heap allocations (Vec, str, etc.) cannot be `Copy`.

### 7.5 Borrowing Rules

The same invariants as Rust apply, enforced at compile time:

- A value can have either one mutable reference OR any number of immutable references at a time.
- References cannot outlive the value they borrow from.
- These rules are checked by the compiler through inferred regions and borrow analysis.

---

## 8. Lifetime and Region Inference

### 8.1 Overview

Lifetimes in Crisp are inferred through region analysis. The compiler assigns each value and reference to a region (a scope of validity) and checks that all borrows respect region containment.

### 8.2 Scope Binding (Phase 1)

Every value is tied to its lexical scope. When the scope ends, the value is dropped and memory is freed. This is deterministic — every allocation has exactly one deallocation point known at compile time.

```rust
process() = {
    data := make_vec()      -- allocated here
    transform(data)
    -- data dropped here, memory freed deterministically
}
```

### 8.3 Flow Tracking (Phase 2)

For values that move between scopes, the compiler tracks the unique owner through the call graph:

```rust
build() = {
    data := make_vec()
    result := transform(data)   -- ownership moves to result
    -- data is no longer valid
    result                       -- ownership moves to caller
}
```

### 8.4 Borrow Resolution (Phase 3)

For references, the compiler builds a borrow graph and checks that all regions are properly nested:

```rust
first(data) = {
    ref := data[0]      -- ref borrows from data
    ref                  -- compiler ensures data outlives ref
}
```

### 8.5 Explicit Lifetimes (Rare)

In the rare case where the compiler cannot resolve lifetimes (typically when multiple input references compete for the output lifetime), explicit annotation is required using tick syntax:

```rust
-- Compiler cannot determine which input the return borrows from
-- ERROR: ambiguous lifetime. Annotate with tick syntax.

-- Fix: both x and y must live as long as the return value
longest('a: x, 'a: y) = if x.len() > y.len() then x else y
```

Tick syntax: `'name: param` means "this parameter lives in region `name`." Shared region names mean the parameters must share a minimum lifetime.

### 8.6 Deterministic Deallocation Guarantee

If a Crisp program compiles, every heap allocation has a statically known deallocation point. There are no runtime decisions about when to free memory. No garbage collector runs. No reference counts are incremented or decremented (unless explicitly opted in — see §8.7).

### 8.7 Escape Hatch: Explicit Reference Counting

For data structures that genuinely require shared ownership (graphs, observer patterns, complex self-referential structures), explicit reference counting is available:

```rust
node := rc(TreeNode { value: 1, children: [] })
shared_ref := node.clone()   -- increments reference count
```

`rc()` wraps a value in a reference-counted container. This is never inserted by the compiler — it is always an explicit programmer decision.

For concurrent shared ownership:

```rust
counter := arc(Mutex.new(0))
```

---

## 9. Error Handling

### 9.1 Ambient Error Propagation

Functions do not declare their error types. The compiler infers fallibility from the function body. If a function calls any operation that can fail, the function itself becomes fallible, and errors propagate automatically to the caller.

```rust
read_config(path) = {
    text := fs.read(path)        -- can fail (IoError)
    config := parse(text)        -- can fail (ParseError)
    config.validate()            -- can fail (ValidationError)
    config
}

-- Inferred signature:
-- read_config(path: &str) -> Config ! IoError | ParseError | ValidationError
```

The `!` in the revealed signature means "can fail with." Error types are automatically unioned.

### 9.2 Error Handling with `catch`

Callers can handle errors explicitly using `catch`:

```rust
main() = {
    cfg := read_config("app.toml") catch err -> {
        log("config error: {err}")
        Config.default()
    }
    run(cfg)
}
```

`catch` intercepts the error and provides a recovery path. The `catch` block must return the same type as the success path.

### 9.3 Selective Catching

Catch specific error types:

```rust
cfg := read_config("app.toml")
    catch IoError -> Config.default()
    catch ParseError(e) -> panic("bad config: {e}")
```

### 9.4 Explicit Error Declaration

You can explicitly constrain which errors a function may produce:

```rust
read_config(path) -> Config ! IoError | ParseError = {
    ...
    -- If the body can produce an error not in the declared set,
    -- the compiler reports an error.
}
```

### 9.5 The `throw` Keyword

Create errors explicitly:

```rust
validate(config) = {
    if config.port == 0 then throw ValidationError("port cannot be 0")
    if config.host.is_empty() then throw ValidationError("host required")
    config
}
```

### 9.6 Non-Fallible Functions

A function that makes no fallible calls is inferred as non-fallible. You can assert this explicitly:

```rust
-- The `!never` annotation asserts this function cannot fail.
-- If the body contains a fallible call, the compiler errors.
add(x, y) -> int !never = x + y
```

### 9.7 Panic

For unrecoverable errors, `panic` terminates the program:

```rust
critical_init() = {
    db := connect_db() catch _ -> panic("cannot start without database")
    db
}
```

Panics are not catchable. They are not part of the error type system.

---

## 10. Pattern Matching

### 10.1 Match Expressions

```rust
eval(expr) = match expr {
    Expr.Literal(n) -> n
    Expr.Add(a, b) -> eval(a) + eval(b)
    Expr.Mul(a, b) -> eval(a) * eval(b)
    Expr.Neg(inner) -> -eval(inner)
}
```

### 10.2 Exhaustiveness

The compiler verifies that match expressions are exhaustive. Missing cases produce a compile error listing the unhandled variants.

### 10.3 Pattern Types

```rust
-- Literal patterns
match x {
    0 -> "zero"
    1 -> "one"
    _ -> "other"
}

-- Tuple patterns
match pair {
    (0, 0) -> "origin"
    (x, 0) -> "x-axis at {x}"
    (0, y) -> "y-axis at {y}"
    (x, y) -> "({x}, {y})"
}

-- Struct patterns
match point {
    { x: 0.0, y: 0.0 } -> "origin"
    { x, y } -> "({x}, {y})"
}

-- Nested patterns
match tree {
    Node { left: Leaf(a), right: Leaf(b) } -> a + b
    Node { left, right } -> count(left) + count(right)
    Leaf(v) -> 1
}

-- Or patterns
match status {
    200 | 201 | 204 -> "success"
    400 | 404 -> "client error"
    _ -> "other"
}

-- Guard clauses
match value {
    x if x < 0 -> "negative"
    x if x > 100 -> "large"
    x -> "normal: {x}"
}

-- Binding with @
match list {
    [first, second, ..rest @ _] if rest.len() > 3 -> "long list"
    _ -> "short list"
}
```

### 10.4 If-Let

```rust
if some(value) := maybe_result {
    process(value)
}

if Ok(data) := fetch() {
    use(data)
} else {
    fallback()
}
```

---

## 11. Concurrency

### 11.1 Async / Await

```rust
fetch_data(url) = async {
    response := http.get(url).await
    body := response.text().await
    parse(body)
}

main() = async {
    data := fetch_data("https://api.example.com").await
    process(data)
}
```

### 11.2 Spawn

Launch concurrent tasks:

```rust
main() = async {
    task1 := spawn fetch_data("https://a.com")
    task2 := spawn fetch_data("https://b.com")

    (result1, result2) := await_all(task1, task2)
    merge(result1, result2)
}
```

### 11.3 Ownership in Concurrency

The ownership inference extends to concurrent contexts:

- Values moved into a `spawn` block are transferred to the new task.
- Shared access across tasks requires explicit `shared()` or `arc()`.
- The compiler rejects data races at compile time.

```rust
-- Moved into task (inferred)
data := make_data()
spawn async {
    process(data)    -- data moved here, not accessible outside
}

-- Shared state must be explicit
counter := arc(Mutex.new(0))
tasks := (0..10).map(|_| {
    c := counter.clone()
    spawn async {
        c.lock().update(|v| v + 1)
    }
})
await_all(tasks)
```

### 11.4 Channels

```rust
(tx, rx) := channel<int>()

spawn async {
    for i in 0..10 {
        tx.send(i).await
    }
    tx.close()
}

for value in rx {
    log("received: {value}")
}
```

---

## 12. Module System

### 12.1 File-Based Modules

Module structure mirrors file structure. No `mod` declarations needed:

```rust
project/
  main.crp
  math/
    vector.crp
    matrix.crp
  io/
    reader.crp
```

Each file is a module. The directory is a namespace.

### 12.2 Visibility

Everything is private by default. Use `pub` to export:

```rust
-- file: math/vector.crp

type Vec3 = { x: float, y: float, z: float }   -- private type

pub type Vec3 = { x: float, y: float, z: float }  -- public type

pub add(a: Vec3, b: Vec3) = Vec3 {
    x: a.x + b.x
    y: a.y + b.y
    z: a.z + b.z
}

helper(v) = ...   -- private to this module
```

### 12.3 Imports

```rust
use math.vector { Vec3, add }
use math.matrix { Mat4, multiply as mat_mul }
use io.reader { * }           -- import all public items (discouraged)
use std.collections { HashMap, HashSet }
```

### 12.4 Re-exports

```rust
-- file: math/mod.crp (or math.crp)
pub use math.vector { Vec3, add }
pub use math.matrix { Mat4 }
```

---

## 13. Memory Model

### 13.1 Stack and Heap

- Primitives and small value types live on the stack.
- Dynamically-sized types (Vec, str, Map, etc.) have stack-allocated metadata (pointer, length, capacity) with heap-allocated contents.
- The compiler decides placement. There is no manual `new` or `malloc`.

### 13.2 Deterministic Drop

Every value has a single, statically-known drop point. When a value goes out of scope or is moved, its destructor runs and memory is freed. This is deterministic — the same execution path always frees memory at the same program point.

Drop order within a scope is reverse declaration order (last declared, first dropped).

### 13.3 Custom Destructors

```rust
type Connection = { handle: RawHandle, name: str }

impl Drop for Connection = {
    drop(self) = {
        log("closing connection: {self.name}")
        raw_close(self.handle)
    }
}
```

### 13.4 Move Semantics

When a value is moved, the source binding becomes invalid:

```rust
a := make_large_struct()
b := a                     -- a is moved to b
-- a is no longer valid here. Using a is a compile error.
```

### 13.5 Clone

For explicit duplication of heap data:

```rust
a := vec![1, 2, 3]
b := a.clone()       -- deep copy, both a and b are valid
```

### 13.6 Memory Safety Guarantees

If a Crisp program compiles, the following are guaranteed:

- No use-after-free.
- No double-free.
- No dangling references.
- No data races.
- No null pointer dereference (no null — use `?T` / Option).
- No buffer overflows (bounds-checked indexing).
- Deterministic deallocation of all resources.

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

C functions are inherently unsafe. Calls must be wrapped in `unsafe` blocks:

```rust
allocate(n) = unsafe {
    ptr := malloc(n)
    if ptr.is_null() then panic("allocation failed")
    ptr
}
```

### 14.2 Calling Rust

Since Crisp targets LLVM, Rust interop is direct:

```rust
extern "rust" {
    -- Import a Rust function
    rust_lib.compute_hash(data: &[u8]) -> u64
}
```

### 14.3 Exporting from Crisp

```rust
-- Export a C-compatible function
pub extern "C" crisp_add(a: i32, b: i32) -> i32 = a + b
```

### 14.4 Unsafe Blocks

Certain operations require `unsafe`:

- Calling foreign functions.
- Dereferencing raw pointers.
- Accessing mutable global state.

```rust
unsafe {
    ptr := raw_pointer_cast(data)
    *ptr = 42
}
```

Unsafe blocks are a code smell. They should be minimal, well-documented, and encapsulated behind safe APIs.

---

## 15. Standard Library Overview

### 15.1 Core Types

```rust
std.option    -- ?T (Option: some / none)
std.result    -- Result<T, E> (Ok / Err) — used for explicit error typing
std.string    -- str, string builder, formatting
std.vec       -- Vec<T>, growable arrays
std.map       -- HashMap<K, V>, BTreeMap<K, V>
std.set       -- HashSet<T>, BTreeSet<T>
```

### 15.2 IO

```rust
std.io        -- read, write, stdin, stdout, stderr
std.fs        -- file system operations
std.net       -- TCP, UDP, DNS
std.http      -- HTTP client/server (async)
```

### 15.3 Concurrency

```rust
std.async     -- async runtime, spawn, await_all
std.sync      -- Mutex, RwLock, Arc, channels
std.atomic    -- atomic types and operations
```

### 15.4 Traits

```rust
std.traits.Show          -- string representation
std.traits.Eq            -- equality comparison
std.traits.Ord           -- ordering
std.traits.Hash          -- hashing
std.traits.Clone         -- deep copy
std.traits.Copy          -- bitwise copy (marker)
std.traits.Drop          -- custom destructor
std.traits.Default       -- default values
std.traits.Into / From   -- type conversion
std.traits.Iterator      -- iteration protocol
std.traits.Add / Sub / Mul / Div  -- operator overloading
```

---

## 16. Tooling: `reveal`

### 16.1 Overview

`reveal` is the companion tool to `crispc`. It surfaces all implicit information that the compiler infers, making Crisp code fully transparent without requiring annotations in source.

### 16.2 Commands

| Command | Description |
|---------|-------------|
| `reveal types <file>` | Show inferred type signatures for all functions |
| `reveal ownership <file>` | Show borrow/move/copy decisions per parameter |
| `reveal lifetimes <file>` | Show inferred region annotations |
| `reveal errors <file>` | Show inferred error types per function |
| `reveal traits <file>` | Show which traits each type satisfies |
| `reveal memory <file>` | Show allocation/deallocation map per function |
| `reveal expand <file>` | Output fully-annotated source (equivalent verbose form) |
| `reveal diff <file>` | Show side-by-side: source vs. fully-annotated |

### 16.3 Example Output

Source (`app.crp`):

```rust
read_config(path) = {
    text := fs.read(path)
    config := parse(text)
    config.validate()
    config
}

greet(name) = "hello " ++ name
```

`reveal types app.crp`:

```rust
read_config(path: &str) -> Config ! IoError | ParseError | ValidationError
greet(name: &str) -> str
```

`reveal ownership app.crp`:

```rust
read_config:
    path: &str            [borrow — read only]
    text: own str         [move from fs.read, consumed by parse]
    config: own Config    [move from parse, returned]

greet:
    name: &str            [borrow — read only]
```

`reveal memory app.crp`:

```rust
read_config:
    text:   str      [alloc line 2, move line 3]
    config: Config   [alloc line 3, return line 5]

greet:
    <return>: str    [alloc line 1, return line 1]
```

### 16.4 LSP Integration

`reveal` powers the Language Server Protocol integration, providing:

- Inline type hints (shown as ghost text in editors).
- Hover information showing full inferred signatures.
- Ownership annotations on mouse-over.
- Error type propagation visualization.
- Memory lifetime visualization.

---

## 17. Compiler Architecture

### 17.1 Compilation Pipeline

```rust
Source (.crp)
    │
    ▼
┌─────────────┐
│   Lexer     │  Tokenization
└─────┬───────┘
      ▼
┌─────────────┐
│   Parser    │  AST generation (brace-delimited, expression-based)
└─────┬───────┘
      ▼
┌─────────────┐
│  Name       │  Resolve imports, build symbol table
│  Resolution │
└─────┬───────┘
      ▼
┌─────────────┐
│  Type       │  Hindley-Milner inference + constraint solving
│  Inference  │
└─────┬───────┘
      ▼
┌─────────────┐
│  Ownership  │  Usage analysis → ownership assignment
│  Inference  │
└─────┬───────┘
      ▼
┌─────────────┐
│  Borrow     │  Region inference, borrow graph validation
│  Checker    │
└─────┬───────┘
      ▼
┌─────────────┐
│  Error      │  Infer fallibility, union error types
│  Inference  │
└─────┬───────┘
      ▼
┌─────────────┐
│  MIR        │  Mid-level IR: drop insertion, monomorphization
│  Generation │
└─────┬───────┘
      ▼
┌─────────────┐
│  LLVM IR    │  Code generation
│  Generation │
└─────┬───────┘
      ▼
┌─────────────┐
│  LLVM       │  Optimization and native code emission
│  Backend    │
└─────┬───────┘
      ▼
  Native Binary
```

### 17.2 Error Reporting

When inference fails, the compiler provides:

1. The specific constraint that could not be solved.
2. The inferred state so far (what it does know).
3. A concrete suggestion for the minimal annotation needed to resolve ambiguity.

Example:

```rust
ERROR [E0042]: ownership ambiguity in `transfer`
  --> lib.crp:15:5
   |
15 |     process(data)
   |             ^^^^ `data` could be borrowed or moved here
   |
   = note: `process` accepts both &T and own T
   = help: annotate to disambiguate:
           process(&data)    -- borrow, data remains valid
           process(own data) -- move, data consumed
```

---

## 18. Package Management

### 18.1 Project Structure

```rust
my_project/
    crisp.toml        -- project manifest
    src/
        main.crp      -- binary entry point
        lib.crp       -- library root
        util/
            helpers.crp
    tests/
        test_util.crp
    bench/
        bench_main.crp
```

### 18.2 Manifest (`crisp.toml`)

```toml
[package]
name = "my_project"
version = "0.1.0"
edition = "2026"

[dependencies]
http = "1.2"
json = { version = "0.5", features = ["pretty"] }
my_lib = { git = "https://github.com/user/my_lib" }

[dev-dependencies]
test_utils = "0.3"
```

### 18.3 Build Commands

```rust
crispc build             -- compile project
crispc run               -- compile and run
crispc test              -- run tests
crispc bench             -- run benchmarks
crispc check             -- type-check without codegen (fast)
reveal expand src/       -- show fully-annotated source for entire project
```

---

## 19. Testing

```rust
-- In tests/test_math.crp or inline with #[test]

test "addition works" = {
    assert_eq(add(2, 3), 5)
}

test "vector magnitude" = {
    v := Vec2.new(3.0, 4.0)
    assert_approx(v.magnitude(), 5.0, epsilon: 0.001)
}

test "parse failure" = {
    result := parse("invalid")
    assert_err(result)
    assert_match(result, Err(ParseError(_)))
}

test "ownership transfer" = {
    data := vec![1, 2, 3]
    consume(data)
    -- `data` should be invalid here; this is a compile-time check,
    -- not a runtime test. The compiler prevents using `data` after move.
}
```

---

## 20. Complete Example

```rust
-- file: src/main.crp

use std.fs
use std.http { Server, Request, Response }
use std.json

type Config = {
    host: str = "localhost"
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
        "/" -> Response.ok("welcome to crisp server")
        "/stats" -> {
            count := state.request_count.load()
            Response.ok(json.encode({ requests: count }))
        }
        "/health" -> Response.ok("ok")
        _ -> Response.not_found("404: {req.path}")
    }
}

pub main() = async {
    config := read_config("config.json")
    state := AppState {
        config: config
        request_count: arc(Atomic.new(0))
    }

    server := Server.bind("{config.host}:{config.port}").await

    log("listening on {config.host}:{config.port}")

    server.serve(|req| handle_request(state, req)).await
}
```

`reveal expand` output for the same code would show every inferred type, lifetime, ownership decision, and error type — equivalent to what you'd write in Rust.

---

## Appendix A: Grammar (EBNF Summary)

```ebnf
program        = { item } ;
item           = function | type_def | trait_def | shape_def | impl_block
               | use_decl | const_def | extern_block ;

function       = [ "pub" ] name "(" [ params ] ")" [ "->" type ] [ error_type ] "=" expr ;
params         = param { "," param } ;
param          = [ lifetime ":" ] [ "own" | "&" | "&mut" ] name [ ":" type ] ;

type_def       = "type" name [ generics ] "=" type_body ;
type_body      = struct_body | enum_body ;
struct_body    = "{" field { "," field } "}" ;
field          = name ":" type [ "=" expr ] ;
enum_body      = { "|" variant } ;
variant        = name [ "(" type { "," type } ")" ] ;

trait_def      = "trait" name "=" "{" { trait_item } "}" ;
trait_item     = name "(" params ")" [ "->" type ] [ "=" expr ] ;

shape_def      = "shape" name "=" "{" { shape_field } "}" ;
shape_field    = name ":" type | name "(" params ")" "->" type ;

impl_block     = "impl" [ trait "for" ] type [ "=" "{" { function } "}" ] ;

use_decl       = "use" path [ "{" imports "}" ] ;
const_def      = name "=" literal ;
extern_block   = "extern" string_lit "{" { extern_fn } "}" ;

expr           = literal | name | block | if_expr | match_expr | loop_expr
               | for_expr | while_expr | call | method_call | field_access
               | lambda | binary_op | unary_op | assign | bind
               | pipe | return_expr | break_expr | throw_expr
               | async_expr | await_expr | spawn_expr | unsafe_block ;

block          = "{" { statement } expr "}" ;
statement      = bind | assign | expr ;
bind           = pattern ":=" expr | pattern "mut:=" expr ;
assign         = name "=" expr ;

if_expr        = "if" expr "then" expr "else" expr
               | "if" expr block [ "else" ( if_expr | block ) ] ;

match_expr     = "match" expr "{" { match_arm } "}" ;
match_arm      = pattern [ "if" expr ] "->" expr ;

for_expr       = "for" pattern "in" expr block ;
while_expr     = "while" expr block ;
loop_expr      = "loop" block ;

lambda         = "|" [ params ] "|" expr ;
call           = expr "(" [ args ] ")" ;
pipe           = expr "|>" expr ;

error_type     = "!" type { "|" type } ;
lifetime       = "'" name ;
generics       = "<" name { "," name } ">" ;

type           = name | tuple_type | array_type | slice_type | fn_type
               | option_type | ref_type | generic_type | shape_constraint ;
option_type    = "?" type ;
ref_type       = "&" [ "mut" ] type ;
```

---

## Appendix B: Comparison with Rust

| Feature | Rust | Crisp |
|---------|------|-------|
| Type annotations | Required on function signatures | Inferred, optional |
| Lifetime annotations | Required when ambiguous | Inferred, tick-syntax for rare cases |
| Ownership declaration | Explicit (&, &mut, move) | Inferred from usage |
| Error handling | Result<T,E> + ? operator | Ambient propagation + catch |
| Trait implementation | Explicit impl blocks | Explicit but with shorthand opt-in |
| Generic bounds | Explicit where clauses | Inferred from body |
| Semicolons | Required | Not used |
| `let` keyword | Required | `:=` binding |
| `fn` keyword | Required | Not used |
| `pub` keyword | Required for visibility | Same |
| Brace blocks | Required | Required |
| Tooling for types | rust-analyzer | reveal |
| GC / RC | Explicit (Rc, Arc) | Explicit (rc, arc) — never automatic |
| Safety guarantees | Full | Equivalent |
| Backend | LLVM | LLVM |

---

*This specification is a living document. Version 0.1.0-draft.*
