//! Const generic trait parameters referenced in method signatures.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<const N: usize> {
    fn arr(&self) -> [u8; N];
    fn total(&self) -> usize;
}

struct X;
struct Y;

impl Tr<3> for X {
    fn arr(&self) -> [u8; 3] {
        [1, 2, 3]
    }
    fn total(&self) -> usize {
        6
    }
}

impl Tr<3> for Y {
    fn arr(&self) -> [u8; 3] {
        [4, 5, 6]
    }
    fn total(&self) -> usize {
        15
    }
}

fn main() {
    use crate::_TrTuple2;

    let t = (X, Y);
    assert_eq!(t.arr(), ([1, 2, 3], [4, 5, 6]));
    assert_eq!(t.total(), (6, 15));
}
