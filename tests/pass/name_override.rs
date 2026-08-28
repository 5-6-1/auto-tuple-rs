//! `name = "X"` overrides the helper trait prefix: `XTuple2` / `XTuple2All`.

use auto_tuple::auto_tuple;

#[auto_tuple(name = "MyTr")]
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
    use crate::MyTrTuple2;

    let t = (X(1), Y(2));
    assert_eq!(t.f(), (1, 2));
}
