# Changelog

> User-visible feature and behavior changes. English, one entry per release.

## Unreleased

- **Generated helper-trait params now avoid method-local generic params**
  (`fn f<__T0>` no longer collides with the element param `__T0` -> E0403).
- **`name = "X"` option** overrides the helper-trait name prefix
  (`XTuple2` / `XTuple2All`).
- **Associated-type bounds are whitelisted by single-segment name only**:
  a same-named local trait (`my::Clone`) is no longer kept on the helper
  trait.

## 0.1.1 (2026-08-27)

- **`pub(crate)` visibility override** for the generated helper traits
  (`#[auto_tuple(pub(crate))]`); default stays "inherit the original trait".
- **Zero-boilerplate within the defining module**: the helper trait is found
  by method resolution automatically, no `use` needed (cross-module calls
  still require an import, as with any trait method).
- Docs: internal-name policy (§7.1), README example fixed to runnable form.

## 0.1.0 (2026-08-27)

- **Initial release** of `#[auto_tuple]`: element-wise tuple-ization for
  trait items.
  - Arity range `2..=12` by default (configurable, incl. 0/1 tuples).
  - Item selection (`#[auto_tuple(foo, MAX, 2..=4)]`); methods by default,
    assoc consts/types only when named.
  - Two helper-trait shapes: **shared** (original params lifted, shared by
    all elements) and **All** (per-element parameter groups, any shape:
    multiple type params, lifetimes, const params; independent instantiation
    per element).
  - Full signature support: `&self`/`&mut self`/`self`, associated fns,
    generic methods, `where` clauses, lifetimes, `async fn`, `unsafe fn`,
    `impl Trait` returns, `Self` in return types recursively, `Self`-shaped
    parameters (unpacked per element), associated consts/types with
    whitelisted bounds.
  - Clear `compile_error!` diagnostics for unsupported shapes.
