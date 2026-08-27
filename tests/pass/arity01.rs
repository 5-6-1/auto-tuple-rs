//! Arity 0 and 1: the unit tuple is a no-op, the 1-tuple forwards to one element.

use auto_tuple::auto_tuple;

#[auto_tuple(0..=1)]
trait Tr {
    fn zero() -> usize;
    fn get(&self) -> usize;
}

struct X(usize);

impl Tr for X {
    fn zero() -> usize {
        7
    }
    fn get(&self) -> usize {
        self.0
    }
}

fn main() {
    use crate::{_TrTuple0, _TrTuple1};

    // 0-tuple: nothing to call, unit results.
    assert_eq!(<()>::zero(), ());
    assert_eq!(<()>::get(&()), ());

    // 1-tuple: forwards to the single element.
    let t = (X(3),);
    assert_eq!(t.get(), (3,));
    assert_eq!(<() as _TrTuple0>::zero(), ());
    assert_eq!(<(X,) as _TrTuple1<X>>::zero(), (7,));
}
