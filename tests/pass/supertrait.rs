//! Super traits are not redeclared on the helper trait; default bodies
//! calling super-trait methods are inherited through the elements.

use auto_tuple::auto_tuple;

trait Base {
    fn base(&self) -> usize;
}

#[auto_tuple]
trait Tr: Base {
    fn derived(&self) -> usize {
        self.base() + 1
    }
    fn own(&self) -> usize;
}

struct X;
struct Y;

impl Base for X {
    fn base(&self) -> usize {
        10
    }
}

impl Base for Y {
    fn base(&self) -> usize {
        20
    }
}

impl Tr for X {
    fn own(&self) -> usize {
        1
    }
}

impl Tr for Y {
    fn own(&self) -> usize {
        2
    }
}

fn main() {
    use crate::_TrTuple2;

    let t = (X, Y);
    assert_eq!(t.derived(), (11, 21));
    assert_eq!(t.own(), (1, 2));
}
