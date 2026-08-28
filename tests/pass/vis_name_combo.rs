//! `pub(crate)` + `name = "X"` compose: `XTuple2` exists and is crate-local.

use auto_tuple::auto_tuple;

#[auto_tuple(pub(crate), name = "Combo")]
trait Tr {
    fn f(&self) -> usize;
}

struct X(usize);
struct Y(usize);

impl Tr for X {
    fn f(&self) -> usize {
        self.0
    }
}
impl Tr for Y {
    fn f(&self) -> usize {
        self.0
    }
}

fn main() {
    use crate::ComboTuple2;

    assert_eq!((X(1), Y(2)).f(), (1, 2));
}
