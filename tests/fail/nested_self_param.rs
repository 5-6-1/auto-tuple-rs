//! `Self` nested inside a generic container is not supported in parameters.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn f(x: Box<Self>);
}

struct X;
struct Y;

impl Tr for X {
    fn f(_x: Box<Self>) {}
}

impl Tr for Y {
    fn f(_x: Box<Self>) {}
}

fn main() {}
