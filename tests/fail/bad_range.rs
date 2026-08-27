//! An inverted range is rejected.

use auto_tuple::auto_tuple;

#[auto_tuple(5..=2)]
trait Tr {
    fn foo(&self);
}

struct X;

impl Tr for X {
    fn foo(&self) {}
}

fn main() {}
