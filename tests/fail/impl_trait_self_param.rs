//! `impl Trait` parameters mentioning `Self` are rejected: a single value
//! cannot satisfy the per-element `Item` constraints.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn f(&self, x: impl Iterator<Item = Self>);
}

struct X;

impl Tr for X {
    fn f(&self, _x: impl Iterator<Item = Self>) {}
}

fn main() {}
