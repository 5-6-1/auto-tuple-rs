//! Selecting a trait item that does not exist is an error.

use auto_tuple::auto_tuple;

#[auto_tuple(does_not_exist)]
trait Tr {
    fn foo(&self);
}

struct X;

impl Tr for X {
    fn foo(&self) {}
}

fn main() {}
