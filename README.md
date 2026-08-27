# auto_tuple

Element-wise tuple-ization for Rust traits: a `#[auto_tuple]` attribute macro
that generates, for each requested arity, a helper trait whose items forward
element-wise to every element of an N-tuple.

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

impl Tr for X { /* ... */ }
impl Tr for Y { /* ... */ }

// Generated for arity 2 (among others):
//
//   trait _TrTuple2<__T0, __T1> where __T0: Tr, __T1: Tr { ... }
//   impl<__T0: Tr, __T1: Tr> _TrTuple2<__T0, __T1> for (__T0, __T1) { ... }

fn main() {
    use auto_tuple::_TrTuple2; // helper trait must be in scope

    let mut t = (X(1), Y(2));
    t.bump();                              // (X(2), Y(3))
    assert_eq!(t.0.0, 2);
    assert_eq!(<(X, Y)>::zero(), (1, 2));  // associated fn forwarding
    assert_eq!(<(X, Y)>::MAX, (5, 50));    // assoc const forwarding
    // <(X, Y) as _TrTuple2<X, Y>>::Output == (X::Output, Y::Output)
}
```

## Core semantics

Every selected trait item is tuple-ized element-wise:

```rust
(x, y).foo(a, b)  == (x.foo(a, b), y.foo(a, b))
(A, B)::MAX       == (A::MAX, B::MAX)
(A, B)::Output    == (A::Output, B::Output)
```

There is no other semantics: no arithmetic merging, no short-circuiting.

## Usage

```rust
#[auto_tuple]                 // default arities 2..=12, all methods
#[auto_tuple(2..=12)]         // explicit arity range
#[auto_tuple(0..=1)]          // arities 0 and 1
#[auto_tuple(foo, 2..=4)]     // only the `foo` method
#[auto_tuple(MAX, Output)]    // only assoc const `MAX` and assoc type `Output`
```

- Ranges follow Rust range semantics (`2..12` excludes 12, `2..=12` includes it).
- Without an explicit item list all **methods** are processed; associated
  consts and types are processed only when explicitly named.
- The helper traits are generated in the same module as the original trait and
  share its visibility; bring them into scope with `use` to call the tuple-ized
  methods.

## Two shapes: shared and All

Whether a trait lands in the **shared** or the **All** shape is decided by
whether any selected item's signature references an original trait generic
parameter:

| | Shared | All |
|---|---|---|
| Trigger | signature references an original generic param | no reference |
| Helper trait | `_TrTuple2<__T0, __T1, T> where __T0: Tr<T>, __T1: Tr<T>` | `_TrTuple2All<__T0, __T1, __TA0, __TA1> where __T0: Tr<__TA0>, __T1: Tr<__TA1>` |
| Element params | shared `T` | independent `__TA0`/`__TA1` (`A: Tr<i32>, B: Tr<String>` allowed) |
| Arity 0/1 names | `_TrTuple0` / `_TrTuple1` | `_TrTuple0All` / `_TrTuple1All` |

Traits without generic parameters always use the plain name and an
unparameterized element bound (`__T0: Tr`).

## Supported / unsupported

Supported:

- `&self` / `&mut self` / `self` receivers and associated functions
- generic methods, `where` clauses, lifetimes (including elided lifetime
  returns such as `fn name(&self) -> &str`)
- `async fn` (sequential await) and `unsafe fn` / `unsafe trait`
- `Self` anywhere in return types, recursively (`&Self`, `Box<Self>`,
  `Vec<Self>`, `Self::Output`, ...)
- `Self`, `&Self`, `&mut Self`, `Self::Assoc` in **parameters** (unpacked per
  element)
- associated consts and types (incl. bounds such as `type Output: Clone;`)

Unsupported (clear `compile_error!`):

- `Self` nested inside a generic container in **parameters** (`Box<Self>`
  params, `Vec<Self>` params)
- generic associated types (GATs)
- `Self` inside where-clause bounds in non-subject position
- auto traits

Left to the compiler (no pre-emption):

- by-value non-`Copy` parameters of arity >= 2 (`(A::g(x), B::g(x))` moves `x`
  twice -> E0382, as with any two calls)
- generic `async fn` if the toolchain rejects them

## Design

See [`docs/design.md`](docs/design.md) for the full design: semantics,
two-track decision procedure, type rewriting rules, error handling and the
edge-case matrix.

## License

MIT
