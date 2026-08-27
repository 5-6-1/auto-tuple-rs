//! Generic associated types are not supported.

use auto_tuple::auto_tuple;

#[auto_tuple(Output)]
trait Tr {
    type Output<T>;
}

struct X;

impl Tr for X {
    type Output<T> = T;
}

fn main() {}
