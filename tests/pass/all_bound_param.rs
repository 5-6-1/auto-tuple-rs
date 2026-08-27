//! All track with a bounded trait param: `Tr<T: Clone>` requires each element
//! `Tr` param to carry the `Clone` bound.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<T: Clone> {
    fn f(&self) -> usize;
}

struct X;
struct Y;

impl Tr<u32> for X {
    fn f(&self) -> usize {
        1
    }
}

impl Tr<String> for Y {
    fn f(&self) -> usize {
        2
    }
}

fn main() {

    assert_eq!((X, Y).f(), (1, 2));
}
