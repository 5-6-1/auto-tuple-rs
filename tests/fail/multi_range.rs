//! Multiple ranges in the attribute are rejected.

use auto_tuple::auto_tuple;

#[auto_tuple(2..=4, 6..=8)]
trait Tr {
    fn foo(&self);
}

struct X;

impl Tr for X {
    fn foo(&self) {}
}

fn main() {}
