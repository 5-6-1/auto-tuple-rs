//! A selected associated type whose bound the element tuple cannot satisfy
//! (`Add`) is dropped from the helper trait; the element side is guaranteed
//! by `A: Tr`. Whitelisted bounds (`Clone`) are kept.

use auto_tuple::auto_tuple;

#[auto_tuple(Out, get, clone_out)]
trait Tr {
    type Out: std::ops::Add<Output = usize> + Clone;
    fn get(&self) -> Self::Out;
    fn clone_out(&self) -> Self::Out
    where
        Self::Out: Clone;
}

struct X;
struct Y;

impl Tr for X {
    type Out = usize;
    fn get(&self) -> usize {
        1
    }
    fn clone_out(&self) -> usize
    where
        Self::Out: Clone,
    {
        3
    }
}

impl Tr for Y {
    type Out = usize;
    fn get(&self) -> usize {
        2
    }
    fn clone_out(&self) -> usize
    where
        Self::Out: Clone,
    {
        4
    }
}

fn main() {
    use crate::_TrTuple2;

    let t = (X, Y);
    assert_eq!(t.get(), (1, 2));
    assert_eq!(t.clone_out(), (3, 4));
}
