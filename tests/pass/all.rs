//! All track: no item references the trait generic param, so elements may
//! implement `Tr` with independent parameters.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<T> {
    fn val(&self) -> usize;
    fn make() -> Box<Self>;
}

struct X(usize);
struct Y(usize);

impl Tr<u32> for X {
    fn val(&self) -> usize {
        self.0
    }
    fn make() -> Box<Self> {
        Box::new(X(1))
    }
}

impl Tr<String> for Y {
    fn val(&self) -> usize {
        self.0
    }
    fn make() -> Box<Self> {
        Box::new(Y(2))
    }
}

fn main() {

    let t = (X(7), Y(8));
    assert_eq!(t.val(), (7, 8));

    let (a, b) = <(X, Y)>::make();
    assert_eq!((a.0, b.0), (1, 2));
}
