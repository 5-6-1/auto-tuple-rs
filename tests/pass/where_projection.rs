//! `where Self::Output: Clone` becomes per-element projections
//! `A::Output: Clone, B::Output: Clone`.

use auto_tuple::auto_tuple;

#[auto_tuple(Output, get)]
trait Tr {
    type Output: Clone;
    fn get(&self) -> Self::Output
    where
        Self::Output: Clone;
}

struct X;
struct Y;

impl Tr for X {
    type Output = usize;
    fn get(&self) -> usize
    where
        Self::Output: Clone,
    {
        1
    }
}

impl Tr for Y {
    type Output = usize;
    fn get(&self) -> usize
    where
        Self::Output: Clone,
    {
        2
    }
}

fn main() {

    assert_eq!((X, Y).get(), (1, 2));
}
