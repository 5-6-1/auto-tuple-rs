//! Custom receivers (`self: Box<Self>`) are not supported.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn f(self: Box<Self>);
}

struct X;

impl Tr for X {
    fn f(self: Box<Self>) {}
}

fn main() {}
