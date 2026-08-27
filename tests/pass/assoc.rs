//! Associated consts, associated types, Self in signatures and selection.

use auto_tuple::auto_tuple;

#[auto_tuple(MAX, MIN, Output, translate, describe)]
trait Tr {
    const MAX: usize;
    const MIN: usize;
    type Output;

    fn translate(&self, n: usize) -> Self::Output;
    fn describe(&self) -> (usize, &'static str);
}

struct X;
struct Y;

impl Tr for X {
    const MAX: usize = 5;
    const MIN: usize = 1;
    type Output = usize;

    fn translate(&self, n: usize) -> usize {
        n + 1
    }
    fn describe(&self) -> (usize, &'static str) {
        (1, "x")
    }
}

impl Tr for Y {
    const MAX: usize = 50;
    const MIN: usize = 10;
    type Output = usize;

    fn translate(&self, n: usize) -> usize {
        n * 2
    }
    fn describe(&self) -> (usize, &'static str) {
        (2, "y")
    }
}

fn main() {

    // Associated const forwarding.
    assert_eq!(<(X, Y)>::MAX, (5, 50));
    assert_eq!(<(X, Y)>::MIN, (1, 10));

    // Associated type is the element-wise tuple.
    fn takes_typed(_: <(X, Y) as _TrTuple2<X, Y>>::Output) {}
    takes_typed((0usize, 0usize));

    // Method using `Self::Output` in its return type.
    let t = (X, Y);
    assert_eq!(t.translate(3), (4, 6));
    assert_eq!(t.describe(), ((1, "x"), (2, "y")));
}
