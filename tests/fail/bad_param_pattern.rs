//! Tuple-destructured parameter patterns are not supported.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn f(&self, (a, b): (usize, usize));
}

struct X;

impl Tr for X {
    fn f(&self, (_a, _b): (usize, usize)) {}
}

fn main() {}
