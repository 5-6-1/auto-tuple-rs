//! A same-named *local* trait bound (`my::Clone`) is not whitelisted: only
//! single-segment std trait names keep their bound on the helper trait.

use auto_tuple::auto_tuple;

mod my {
    pub trait Clone {}
    impl Clone for usize {}
}

#[auto_tuple(Out, get)]
trait Tr {
    type Out: my::Clone;
    fn get(&self) -> Self::Out;
}

struct X;
struct Y;

impl Tr for X {
    type Out = usize;
    fn get(&self) -> usize {
        1
    }
}
impl Tr for Y {
    type Out = usize;
    fn get(&self) -> usize {
        2
    }
}

fn main() {
    let t = (X, Y);
    assert_eq!(t.get(), (1, 2));
}
