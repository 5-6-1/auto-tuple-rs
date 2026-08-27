//! `&mut T` parameters are reborrowed per element; by-value `Copy` params are
//! passed through unchanged.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<T> {
    fn set(&mut self, v: &mut T);
    fn scale(&mut self, v: &mut T) -> T;
    fn echo<X: Copy>(x: X) -> X;
}

struct X(usize);
struct Y(usize);

impl Tr<u32> for X {
    fn set(&mut self, v: &mut u32) {
        *v += 1;
    }
    fn scale(&mut self, v: &mut u32) -> u32 {
        *v *= 2;
        *v
    }
    fn echo<X: Copy>(x: X) -> X {
        x
    }
}

impl Tr<u32> for Y {
    fn set(&mut self, v: &mut u32) {
        *v += 10;
    }
    fn scale(&mut self, v: &mut u32) -> u32 {
        *v *= 3;
        *v
    }
    fn echo<X: Copy>(x: X) -> X {
        x
    }
}

fn main() {

    let mut t = (X(1), Y(2));

    // Sequential reborrows: first element sees the mutation, then the second.
    let mut v = 0u32;
    t.set(&mut v);
    assert_eq!(v, 11);
    assert_eq!(t.scale(&mut v), (22, 66));
    assert_eq!(v, 66);

    // By-value Copy generic param forwards fine.
    assert_eq!(<(X, Y)>::echo(7u32), (7, 7));
}
