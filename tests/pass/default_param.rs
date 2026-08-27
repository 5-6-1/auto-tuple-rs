//! Trait generic params with defaults: lifted into the helper trait with the
//! default retained; impl params drop the default.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<T = i32> {
    fn conv(x: &T) -> T;
    fn get(&self) -> usize;
}

struct X;
struct Y;

impl Tr for X {
    fn conv(x: &i32) -> i32 {
        x + 1
    }
    fn get(&self) -> usize {
        1
    }
}

impl Tr for Y {
    fn conv(x: &i32) -> i32 {
        x * 2
    }
    fn get(&self) -> usize {
        2
    }
}

fn main() {

    let v = 5;
    assert_eq!(<(X, Y)>::conv(&v), (6, 10));
    assert_eq!((X, Y).get(), (1, 2));
}
