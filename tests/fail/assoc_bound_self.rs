//! `Self` inside an associated type bound is rejected.

use auto_tuple::auto_tuple;

#[auto_tuple(Output)]
trait Tr {
    type Output: Into<Self>;
}

struct X;

impl Tr for X {
    type Output = X;
}

fn main() {}
