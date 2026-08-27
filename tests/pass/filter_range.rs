//! Explicit selection with a non-default arity: only `foo` is tuple-ized and
//! only the 3-tuple helper is generated.

use auto_tuple::auto_tuple;

#[auto_tuple(foo, 3..=3)]
trait Tr {
    fn foo(&self) -> usize;
    fn bar(&self) -> usize;
}

struct X;
struct Y;
struct Z;

impl Tr for X {
    fn foo(&self) -> usize {
        1
    }
    fn bar(&self) -> usize {
        100
    }
}

impl Tr for Y {
    fn foo(&self) -> usize {
        2
    }
    fn bar(&self) -> usize {
        200
    }
}

impl Tr for Z {
    fn foo(&self) -> usize {
        3
    }
    fn bar(&self) -> usize {
        300
    }
}

fn main() {
    use crate::_TrTuple3;

    let t = (X, Y, Z);
    assert_eq!(t.foo(), (1, 2, 3));
}
