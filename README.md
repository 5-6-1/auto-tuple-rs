# auto-tuple

[![CI](https://github.com/5-6-1/auto-tuple-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/5-6-1/auto-tuple-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/auto-tuple)](https://crates.io/crates/auto-tuple)
[![docs.rs](https://img.shields.io/docsrs/auto-tuple)](https://docs.rs/auto-tuple)

Element-wise tuple-ization for Rust traits: an `#[auto_tuple]` attribute macro
that generates, for each requested arity, a helper trait whose items forward
element-wise to every element of an N-tuple. Package `auto-tuple`, crate
`auto_tuple`.

```rust
use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn zero() -> usize;
    fn bump(&mut self) -> usize;
    const MAX: usize;
    type Output;
}

struct X(usize);
struct Y(usize);

impl Tr for X {
    fn zero() -> usize { 1 }
    fn bump(&mut self) -> usize { self.0 += 1; self.0 }
    const MAX: usize = 5;
    type Output = u32;
}
impl Tr for Y {
    fn zero() -> usize { 2 }
    fn bump(&mut self) -> usize { self.0 += 1; self.0 }
    const MAX: usize = 50;
    type Output = u64;
}

// Generated for arity 2 (among others):
//
//   trait _TrTuple2<__T0, __T1> where __T0: Tr, __T1: Tr { ... }
//   impl<__T0: Tr, __T1: Tr> _TrTuple2<__T0, __T1> for (__T0, __T1) { ... }

fn main() {
    // No imports needed: the helper trait lives in this module, so method
    // resolution finds it automatically. Zero boilerplate.
    let mut t = (X(1), Y(2));
    assert_eq!(t.bump(), (2, 3));
    assert_eq!(t.0.0, 2);
    assert_eq!(<(X, Y)>::zero(), (1, 2));  // associated fn forwarding
    assert_eq!(<(X, Y)>::MAX, (5, 50));    // assoc const forwarding
    let _: <(X, Y) as _TrTuple2<X, Y>>::Output = (1u32, 2u64);
}
```

The helper traits are generated in the same module as the original trait and
share its visibility. Within that module the tuple-ized methods are callable
directly; **cross-module calls need the helper trait imported**, exactly like
any other trait's methods (`use std::io::Read` to call `.read()`).

## Core semantics

Every selected trait item is tuple-ized element-wise:

```text
(x, y).foo(a, b)  == (x.foo(a, b), y.foo(a, b))
(A, B)::MAX       == (A::MAX, B::MAX)
(A, B)::Output    == (A::Output, B::Output)
```

There is no other semantics: no arithmetic merging, no short-circuiting.

## Usage

```text
#[auto_tuple]                 // default arities 2..=12, all fn/const/type items
#[auto_tuple(2..=12)]         // explicit arity range
#[auto_tuple(0..=1)]          // arities 0 and 1
#[auto_tuple(foo, 2..=4)]     // only the `foo` method
#[auto_tuple(MAX, Output)]    // only assoc const `MAX` and assoc type `Output`
#[auto_tuple(@all_methods)]  // all methods, including associated functions
#[auto_tuple(@all_methods, MAX)] // union: methods plus MAX
#[auto_tuple(-reset)]        // all items except reset
#[auto_tuple(@all, -@all_types)] // all items except associated types
#[auto_tuple(@all_methods, -[reset, clear], 2..=4)]
#[auto_tuple(pub(crate), name = "Reader", @all_ref_methods)]
```

- Ranges follow Rust range semantics (`2..12` excludes 12, `2..=12` includes it).
- **Compile cost**: each arity generates one helper trait + blanket impl, so
  the default `2..=12` emits 11 pairs per annotated trait. For heavy trait
  graphs prefer a tighter range (`#[auto_tuple(2..=4)]`) to keep build times
  down.
- Without a positive selector all **methods, associated constants and
  associated types** are selected, including when only exclusions are given.
- Positive names and families form a union; exclusions are applied last, so
  `@all, -foo, foo` still excludes `foo`. Duplicates are removed. Flat lists
  (`[foo, MAX]`, `-[@all_types, reset]`) are also supported.
- Unknown member names (including exclusions) and unknown families are errors.
  An empty family or a fully excluded set stays empty and generates empty
  helper traits; it never falls back to all members. Unsupported items are
  checked only after selection and are not silently skipped.
- A visibility override (`pub`, `pub(crate)`, ...) applies to the generated
  helper traits; by default they inherit the original trait's visibility.
- The helper traits are generated in the same module as the original trait and
  share its visibility; bring them into scope with `use` to call the tuple-ized
  methods.

### Member selector families

The names and filters match batch-impl's member families:

| Family | Selects |
|---|---|
| `@all` | All methods, associated constants and associated types |
| `@all_methods` / `@all_constants` / `@all_types` | All members of that kind |
| `@all_required` / `@all_default` | Members without / with a default body or value |
| `@all_required_methods` / `@all_default_methods` | Methods filtered by default body |
| `@all_required_constants` / `@all_default_constants` | Constants filtered by default value |
| `@all_required_types` / `@all_default_types` | Associated types filtered by default type |
| `@all_ref_methods` | Methods with `&self` or `&mut self` |
| `@all_value_methods` | Methods with `self`, including typed receivers |
| `@all_static_methods` | Associated functions without a receiver |

A family classifies syntax; it does not enable unsupported language features.
For example, selecting a typed receiver still reports the custom-receiver
diagnostic. A selected default method forwards to each element's implementation.

```rust
use auto_tuple::auto_tuple;

#[auto_tuple(@all_ref_methods, MAX, -reset, 2..=2)]
trait Read {
    fn read(&self) -> usize;
    fn reset(&mut self);
    fn consume(self: Box<Self>); // excluded before receiver validation
    const MAX: usize = 10;
}

struct Reader;
impl Read for Reader {
    fn read(&self) -> usize { 7 }
    fn reset(&mut self) {}
    fn consume(self: Box<Self>) {}
}

assert_eq!((Reader, Reader).read(), (7, 7));
assert_eq!(<(Reader, Reader)>::MAX, (10, 10));
```

### Migrating from 0.1.x

Replace a bare `#[auto_tuple]` with `#[auto_tuple(@all_methods)]` to preserve
the former methods-only default. Existing explicit member lists retain their
meaning. The new default may select unsupported associated types or change
Shared/All analysis when an associated item's bounds mention a trait parameter;
review helper-trait imports and explicit UFCS paths when adopting it.

## Two shapes: shared and All

Whether a trait lands in the **shared** or the **All** shape is decided by
whether any selected item's signature references an original trait generic
parameter:

| | Shared | All |
|---|---|---|
| Trigger | a selected item's signature references an original generic param | no selected item references the original generic params (any shape: multiple type params, lifetimes, const params) |
| Helper trait | `_TrTuple2<__T0, __T1, T> where __T0: Tr<T>, __T1: Tr<T>` | `_TrTuple2All<__T0, __T1, <per-element param groups>> where __T0: Tr<...>, __T1: Tr<...>` |
| Element params | shared original params | one independent param group per element (`A: Tr<i32, S>, B: Tr<String, U>` allowed) |
| Bounds | original params lifted as-is | original type param bounds carry to the element group (`Tr<T: Clone>` → per-element `Clone`) |
| Arity 0/1 names | `_TrTuple0` / `_TrTuple1` | `_TrTuple0All` / `_TrTuple1All` |

Traits without generic parameters always use the plain name and an
unparameterized element bound (`__T0: Tr`).

## Supported / unsupported

Supported:

- `&self` / `&mut self` / `self` receivers and associated functions
- generic methods, `where` clauses, lifetimes (trait-level `'a` is lifted into
  the helper trait; elided-lifetime returns such as `fn name(&self) -> &str`
  work)
- `async fn` (sequential await) and `unsafe fn` / `unsafe trait`
- `impl Trait` (RPIT) returns: each element keeps its own opaque type
  (`fn f() -> impl Iterator<Item = Self>` becomes a tuple of per-element
  opaques)
- `Self` anywhere in return types, recursively (`&Self`, `Box<Self>`,
  `Vec<Self>`, `Self::Output`, ...)
- `Self`, `&Self`, `&mut Self`, `Self::Assoc` in **parameters** (unpacked per
  element)
- associated consts and types (incl. bounds such as `type Output: Clone;`)

Unsupported (clear `compile_error!`):

- `Self` nested inside a generic container in **parameters** (`Box<Self>`
  params, `Vec<Self>` params)
- by-value `impl Trait` parameters (a single value cannot be forwarded to
  multiple elements; use a reference or a `Copy` generic)
- `impl Trait` parameters mentioning `Self` (a single value cannot satisfy the
  per-element `Item` constraints)
- custom receivers (`self: Box<Self>` etc.)
- `+ use<..>` precise capturing in returns (the trait/impl capture lists of
  the per-element opaques are not yet handled)
- generic associated types (GATs)
- `Self` inside where-clause bounds in non-subject position
- auto traits
- multiple arity ranges in one attribute

Left to the compiler (no pre-emption):

- by-value non-`Copy` parameters of arity >= 2 (`(A::g(x), B::g(x))` moves `x`
  twice -> E0382, as with any two calls)
- generic `async fn` if the toolchain rejects them

## Design

See [`docs/design.md`](https://github.com/5-6-1/auto-tuple-rs/blob/master/docs/design.md) for the full design: semantics,
two-track decision procedure, type rewriting rules, error handling and the
edge-case matrix.

## Contributing

See [`docs/development-guide.md`](https://github.com/5-6-1/auto-tuple-rs/blob/master/docs/development-guide.md) for the development
and release conventions, and [`CONTRIBUTING.md`](https://github.com/5-6-1/auto-tuple-rs/blob/master/CONTRIBUTING.md) for quick checks.

## License

MIT — see [`LICENSE`](https://github.com/5-6-1/auto-tuple-rs/blob/master/LICENSE).
