//! Associated type bounds are retained on the helper trait and their
//! element-wise copies are added to the impl's where clause.

use auto_tuple::auto_tuple;

#[auto_tuple(Output, dup)]
trait Tr {
    type Output: Clone;
    fn dup(&self) -> Self::Output;
}

struct X;
struct Y;

impl Tr for X {
    type Output = usize;
    fn dup(&self) -> usize {
        3
    }
}

impl Tr for Y {
    type Output = usize;
    fn dup(&self) -> usize {
        4
    }
}

fn main() {

    let t = (X, Y);
    assert_eq!(t.dup(), (3, 4));

    // The helper trait's associated type is the element-wise tuple and the
    // `Clone` bound holds.
    fn clone_pair<A: Tr, B: Tr>(x: <(A, B) as _TrTuple2<A, B>>::Output) -> <(A, B) as _TrTuple2<A, B>>::Output {
        x.clone()
    }
    let c = clone_pair::<X, Y>((0usize, 0usize));
    assert_eq!(c, (0, 0));
}
