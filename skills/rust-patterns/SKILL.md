---
name: rust-patterns
description: "Reference for intermediate-to-advanced Rust patterns: generics, traits, error handling (thiserror/anyhow), concurrency, smart pointers, macros, API design, async, unsafe, testing, and pattern decision guides. Use when designing Rust APIs, choosing between generics/enums/trait objects, setting up error handling, working with async/concurrency, or need quick trait bound/lifetime reference. Source: Microsoft Rust Training Patterns Book."
---

# Rust Patterns & Engineering How-Tos

Comprehensive reference for intermediate-to-advanced Rust patterns derived from Microsoft's Rust Training program. Covers generics, traits, error handling, concurrency, smart pointers, macros, API design, async, and more.

**Source**: https://microsoft.github.io/RustTraining/rust-patterns-book/

## Trigger
Use this skill when:
- Designing Rust APIs, error types, or generic abstractions
- Choosing between generics, enums, and trait objects
- Working with async Rust, unsafe code, macros, or FFI
- Setting up error handling with thiserror/anyhow
- Implementing smart pointers, interior mutability, or const generics
- Writing tests, benchmarks, or property-based tests
- Need a quick reference for trait bounds, lifetimes, or module visibility

## Quick Decision Guides

### Generics vs Enums vs Trait Objects
```
Known closed set of types → Enum (exhaustive matching, zero cost)
Open set, hot path → Generics (monomorphized, inlined)
Open set, cold path → dyn Trait (vtable dispatch)
Mixed types in one collection → Vec<Box<dyn Trait>>
```

### Error Handling
```
Libraries → thiserror (#[derive(Error)]) — structured, matchable enums
Applications → anyhow (Result<T>) — ergonomic propagation with .context()
```

### Concurrency
```
Simple counter/flag → Atomics
Short critical section → Mutex
Read-heavy → RwLock
Lazy one-time init → OnceLock / LazyLock
Complex state → Actor + Channels
Parallel computation → rayon::par_iter
Background task → thread::spawn
Borrow local data → thread::scope
```

### Smart Pointers
```
Owned heap allocation → Box<T>
Shared ownership (single thread) → Rc<T>
Shared ownership (multi thread) → Arc<T>
Breaking reference cycles → Weak<T>
Interior mutability (single thread) → RefCell<T>
Interior mutability (multi thread) → Mutex<T> / RwLock<T>
Copy-on-write → Cow<T>
Prevent moves (Futures, self-ref) → Pin<T>
Zero-cost wrapper → Newtype pattern
```

---

## Chapter Reference

### Part I: Type-Level Patterns

#### 1. Generics (🟢)
- **Monomorphization**: Compiler generates specialized copies per concrete type — zero runtime cost
- **Code bloat mitigation**: Extract non-generic core into separate fn; use dyn Trait for cold paths
- **Const generics**: `struct Matrix<const ROWS: usize, const COLS: usize>` — compile-time sizes
- **const fn**: Evaluated at compile time — eliminates lazy_static for lookup tables

```rust
// Code bloat mitigation — "outline" pattern
fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let json_value = serde_json::to_value(value)?;  // Generic
    serialize_value(json_value)                      // Non-generic — one copy
}
fn serialize_value(value: Value) -> Result<Vec<u8>, Error> { ... }

// Const generics
struct RegisterBlock<const N: usize> { registers: [u32; N] }
```

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch01-generics-the-full-picture.html

#### 2. Traits (🟡)
- **Associated types**: `type Output;` — one impl per type (vs generics: many impls)
- **GATs (Generic Associated Types)**: `type Iter<'a> where Self: 'a;` — lifetimes in associated types
- **Blanket impls**: `impl<T: Display> ToString for T {}`
- **HRTBs**: `where F: for<'a> Fn(&'a str) -> &'a str` — higher-ranked trait bounds
- **Extension traits**: Add methods to foreign types via `trait FooExt: Foo { fn bar(&self); }`
- **Marker traits**: `Send`, `Sync`, `Unpin`, `Sized` — zero-cost compile-time guarantees

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch02-traits-in-depth.html

#### 3. Newtype & Type-State (🟡)
- **Newtype**: `struct UserId(u64)` — zero-cost type safety, prevents mixing primitives
- **Type-state**: Compile-time state machines via generics — `Connection<Connected>` vs `Connection<Disconnected>`
- **Builder pattern**: Chain methods that transition state types

```rust
// Newtype
struct Meters(f64);
struct Feet(f64);
fn add(a: Meters, b: Meters) -> Meters { Meters(a.0 + b.0) }

// Type-state
struct Connection<S> { stream: TcpStream, _state: PhantomData<S> }
struct Connected;
struct Disconnected;
impl Connection<Disconnected> { fn connect(self) -> Connection<Connected> { ... } }
impl Connection<Connected> { fn send(&mut self, data: &[u8]) { ... } }
```

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch03-the-newtype-and-type-state-patterns.html

#### 4. PhantomData (🔴)
- **Lifetime branding**: `PhantomData<&'a ()>` — tie a type to a lifetime without storing data
- **Variance control**: `PhantomData<fn(T)>` (invariant) vs `PhantomData<T>` (covariant)
- **Drop check**: Ensure ownership semantics without runtime data
- **Unit-of-measure pattern**: Combine with newtypes for dimensional analysis

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch04-phantomdata-types-that-carry-no-data.html

### Part II: Concurrency & Runtime

#### 5. Channels (🟢)
- **mpsc**: Multiple producers, single consumer
- **crossbeam**: Multi-producer, multi-consumer; `select!` macro
- **Actor pattern**: Channel + thread = lightweight actor
- **Backpressure**: `sync_channel` with bounded capacity

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch05-channels-and-message-passing.html

#### 6. Concurrency (🟡)
- **Threads**: `thread::spawn`, `thread::scope` (borrows locals), `thread::Builder`
- **rayon**: `par_iter()` for data parallelism
- **Sync primitives**: Mutex, RwLock, Condvar, OnceLock, Barrier, Atomics
- **Lock-free**: `AtomicBool`, `AtomicUsize`, `Ordering::Acquire/Release`

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch06-concurrency-vs-parallelism-vs-threads.html

#### 7. Closures (🟢)
- **Fn**: Borrows immutably, can be called multiple times
- **FnMut**: Borrows mutably, can be called multiple times
- **FnOnce**: Consumes captured values, called once
- **move closures**: Take ownership of captured values
- **Higher-order functions**: Functions that accept/return closures

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch07-closures-and-higher-order-functions.html

#### 8. Functional vs Imperative (🟡)
- **Combinators**: `map`, `filter`, `fold`, `flat_map`, `collect`
- **Iterator adapters**: Lazy, zero-cost chaining
- **When to use**: Prefer functional for pure transformations, imperative for complex state

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch08-functional-vs-imperative-when-elegance-wins.html

#### 9. Smart Pointers (🟡)
- **Box<T>**: Owned heap allocation, recursive types, trait objects
- **Rc<T>/Arc<T>**: Shared ownership (single/multi-thread)
- **Weak<T>**: Non-owning reference, breaks cycles
- **RefCell<T>**: Interior mutability (runtime borrow checking)
- **Cell<T>**: Copy-based interior mutability
- **Cow<T>**: Copy-on-write — borrow or own
- **Pin<T>**: Prevent moves — required for Futures
- **ManuallyDrop<T>**: Manual destructor control

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch09-smart-pointers-and-interior-mutability.html

### Part III: Systems & Production

#### 10. Error Handling (🟢)
- **thiserror** (libraries): `#[derive(Error)]` for structured enums
- **anyhow** (applications): `Result<T>` with `.context()`, `bail!`, `ensure!`
- **#[from]**: Auto-generates `From<OtherError>` impls
- **.context()**: Adds human-readable wrapper without losing source
- **? operator**: Desugars to `From::from()` + early return

```rust
// Library pattern
#[derive(Error, Debug)]
pub enum DbError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// Application pattern
fn main() -> anyhow::Result<()> {
    let config = read_config("server.toml")
        .context("failed to load config")?;
    Ok(())
}
```

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch10-error-handling-patterns.html

#### 11. Serialization (🟡)
- **serde**: Derive `Serialize`/`Deserialize`
- **Zero-copy**: `&'de str` borrowing from input, `#[serde(borrow)]`
- **Binary**: `repr(C)` for FFI, `#[repr(packed)]`, `bytemuck` for safe casting
- **Enum representations**: Externally tagged (default), internally tagged, adjacently tagged, untagged

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch11-serialization-zero-copy-and-binary-data.html

#### 12. Unsafe Rust (🔴)
- **Five superpowers**: Dereference raw pointers, call unsafe fns, access mutable statics, implement unsafe traits, access union fields
- **Sound abstractions**: Wrap unsafe in safe API — `Vec`, `Box`, `Mutex` do this
- **FFI**: `extern "C"`, `#[no_mangle]`, `#[repr(C)]`
- **UB pitfalls**: Dangling pointers, data races, invalid values, uninitialized memory
- **Arena/slab allocators**: Custom allocation for embedded/performance

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch12-unsafe-rust-controlled-danger.html

#### 13. Macros (🟡)
- **Declarative macros**: `macro_rules!` — pattern matching on tokens, `tt` munching
- **Proc macros**: `#[proc_macro]`, `#[proc_macro_derive]`, `#[proc_macro_attribute]`
- **syn/quote**: Parse and generate Rust code
- **When NOT to use**: When a generic function or trait would suffice

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch13-macros-code-that-writes-code.html

#### 14. Testing (🟢)
- **Unit tests**: `#[cfg(test)] mod tests { #[test] fn ... }`
- **Integration tests**: `tests/` directory
- **Doc tests**: Examples in documentation that compile and run
- **proptest**: Property-based testing — generate inputs, verify invariants
- **criterion**: Statistical benchmarking
- **Mocking**: `mockall`, dependency injection via generics/traits

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch14-testing-and-benchmarking-patterns.html

#### 15. API Design (🟡)
- **Module layout**: `pub mod`, `pub use` for re-exports, `pub(crate)` for internals
- **Ergonomic parameters**: `impl Into<T>`, `AsRef<Path>`, `Option<T>`
- **Feature flags**: `#[cfg(feature = "serde")]` for optional dependencies
- **Parse, don't validate**: Make invalid states unrepresentable
- **Builder pattern**: For complex construction with defaults
- **Workspaces**: Multi-crate projects with shared dependencies

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch15-crate-architecture-and-api-design.html

#### 16. Async (🔴)
- **Futures**: Lazy state machines that don't run unless polled
- **Tokio**: `#[tokio::main]`, `tokio::spawn`, `tokio::select!`
- **Pitfalls**: Blocking in async context, `Send` bound, cancellation safety, Pin
- **Anti-patterns**: `.await` in loop with unbounded concurrency, `Mutex` over async (use `tokio::sync::Mutex`)

📎 **Full chapter**: https://microsoft.github.io/RustTraining/rust-patterns-book/ch16-asyncawait-essentials.html

---

## Quick Reference Cards

### Trait Bounds Cheat Sheet
| Bound | Meaning |
|-------|---------|
| `T: Clone` | Can be `.clone()`d |
| `T: Send` | Can be moved to another thread |
| `T: Sync` | `&T` can be shared between threads |
| `T: 'static` | Contains no non-static references |
| `T: Sized` | Size known at compile time (default) |
| `T: ?Sized` | Size may not be known (`[T]`, `dyn Trait`) |
| `T: Default` | Has `T::default()` |
| `T: Into<U>` | Can be converted to `U` |
| `T: AsRef<U>` | Can be borrowed as `&U` |
| `T: Deref<Target=U>` | Auto-derefs to `&U` |
| `F: Fn(A) -> B` | Callable, borrows immutably |
| `F: FnMut(A) -> B` | Callable, may mutate |
| `F: FnOnce(A) -> B` | Callable exactly once |

### Lifetime Elision Rules
```
Rule 1: Each reference parameter gets its own lifetime
  fn foo(x: &str, y: &str) → fn foo<'a, 'b>(x: &'a str, y: &'b str)

Rule 2: One input lifetime → used for all outputs
  fn foo(x: &str) -> &str → fn foo<'a>(x: &'a str) -> &'a str

Rule 3: &self/&mut self → output gets self's lifetime
  fn foo(&self, x: &str) -> &str → fn foo<'a>(&'a self, x: &str) -> &'a str
```

**Must write explicit lifetimes when**: Multiple input refs + ref output; struct fields with refs; `'static` bounds.

### Module Visibility
```
pub           → visible everywhere
pub(crate)    → visible within the crate
pub(super)    → visible to parent module
(nothing)     → private to current module + children
```

### Common Derive Traits
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct MyType { ... }
```

---

## Pattern Decision Flowchart

```text
Need type safety for primitives?
└── Newtype pattern (Ch 3)

Need compile-time state enforcement?
└── Type-state pattern (Ch 3)

Need a "tag" with no runtime data?
└── PhantomData (Ch 4)

Need to break Rc/Arc reference cycles?
└── Weak<T> / sync::Weak<T> (Ch 9)

Need to wait for a condition without busy-looping?
└── Condvar + Mutex (Ch 6)

Need to handle "one of N types"?
├── Known closed set → Enum
├── Open set, hot path → Generics
├── Open set, cold path → dyn Trait
└── Completely unknown → Any + TypeId (Ch 2)

Need shared state across threads?
├── Simple counter/flag → Atomics
├── Short critical section → Mutex
├── Read-heavy → RwLock
├── Lazy one-time init → OnceLock / LazyLock (Ch 6)
└── Complex state → Actor + Channels

Need to parallelize computation?
├── Collection processing → rayon::par_iter
├── Background task → thread::spawn
└── Borrow local data → thread::scope

Need async I/O or concurrent networking?
├── Basic → tokio + async/await (Ch 16)
└── Advanced → see Async Rust Training

Need error handling?
├── Library → thiserror (#[derive(Error)])
└── Application → anyhow (Result<T>)

Need to prevent a value from being moved?
└── Pin<T> (Ch 9) — required for Futures
```

---

## Source

**Book**: [Rust Patterns & Engineering How-Tos](https://microsoft.github.io/RustTraining/rust-patterns-book/) — Microsoft Rust Training

**GitHub**: https://github.com/microsoft/RustTraining

**Difficulty Legend**:
- 🟢 Fundamentals — core concepts every Rust developer needs
- 🟡 Intermediate — patterns used in production codebases
- 🔴 Advanced — deep language mechanics, revisit as needed

**Total estimated study time**: 30–45 hours for thorough study with exercises.
