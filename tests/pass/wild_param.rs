//! Unnamed (`_`) parameters get a synthetic name so they can be forwarded.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn use_it(&self, _: usize) -> usize;
    fn ignore(_: &'static str) -> usize;
}

struct X;
struct Y;

impl Tr for X {
    fn use_it(&self, _: usize) -> usize {
        1
    }
    fn ignore(_: &'static str) -> usize {
        10
    }
}

impl Tr for Y {
    fn use_it(&self, _: usize) -> usize {
        2
    }
    fn ignore(_: &'static str) -> usize {
        20
    }
}

fn main() {
    use crate::_TrTuple2;

    let t = (X, Y);
    assert_eq!(t.use_it(7), (1, 2));
    assert_eq!(<(X, Y)>::ignore("x"), (10, 20));
}
