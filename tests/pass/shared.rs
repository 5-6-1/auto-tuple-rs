//! Shared track: item signatures reference the original trait generic param.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<T> {
    fn conv(x: &T) -> T;
    fn conv2(x: &T) -> (T, T);
}

struct X;
struct Y;

impl Tr<u32> for X {
    fn conv(x: &u32) -> u32 {
        x + 1
    }
    fn conv2(x: &u32) -> (u32, u32) {
        (x + 1, x + 2)
    }
}

impl Tr<u32> for Y {
    fn conv(x: &u32) -> u32 {
        x * 2
    }
    fn conv2(x: &u32) -> (u32, u32) {
        (x * 2, x * 3)
    }
}

fn main() {
    use crate::_TrTuple2;

    let v = 10;
    assert_eq!(<(X, Y)>::conv(&v), (11, 20));
    assert_eq!(<(X, Y)>::conv2(&v), ((11, 12), (20, 30)));
}
