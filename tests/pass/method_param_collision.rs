//! Generated params avoid method-local generic params (`fn f<__T0>` would
//! otherwise collide with the element param `__T0` -> E0403).

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn f<__T0: Copy>(x: __T0) -> __T0;
    fn g(&self) -> usize;
}

struct X(usize);
struct Y(usize);

impl Tr for X {
    fn f<__T0: Copy>(x: __T0) -> __T0 {
        x
    }
    fn g(&self) -> usize {
        self.0
    }
}

impl Tr for Y {
    fn f<__T0: Copy>(x: __T0) -> __T0 {
        x
    }
    fn g(&self) -> usize {
        self.0
    }
}

fn main() {
    assert_eq!(<(X, Y)>::f(42), (42, 42));
    assert_eq!((X(1), Y(2)).g(), (1, 2));
}
