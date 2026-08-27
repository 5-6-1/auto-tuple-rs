//! All track with associated consts and types: element-wise forwarding under
//! independent `Tr` parameters.

use auto_tuple::auto_tuple;

#[auto_tuple(MAX, Output, get)]
trait Tr<T> {
    const MAX: usize;
    type Output;
    fn get(&self) -> usize;
}

struct X(usize);
struct Y(usize);

impl Tr<u32> for X {
    const MAX: usize = 5;
    type Output = u32;
    fn get(&self) -> usize {
        self.0
    }
}

impl Tr<String> for Y {
    const MAX: usize = 50;
    type Output = String;
    fn get(&self) -> usize {
        self.0
    }
}

fn main() {
    use crate::_TrTuple2All;

    let t = (X(7), Y(8));
    assert_eq!(t.get(), (7, 8));
    assert_eq!(<(X, Y)>::MAX, (5, 50));

    fn typed(_: <(X, Y) as _TrTuple2All<X, Y, u32, String>>::Output) {}
    typed((0u32, String::new()));
}
