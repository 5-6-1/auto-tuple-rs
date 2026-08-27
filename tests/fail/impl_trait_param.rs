//! By-value `impl Trait` parameters cannot be forwarded to multiple elements.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn f(&self, x: impl Clone);
}

struct X;

impl Tr for X {
    fn f(&self, _x: impl Clone) {}
}

fn main() {}
