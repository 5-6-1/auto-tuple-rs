//! Generic methods: method-local type params, bounds and where clauses are
//! forwarded as-is; return types tuple-ize.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn tag<X: Clone>(&self, x: &X) -> X;
    fn convert<T: Copy, U: Copy>(&self, t: T, u: U) -> (T, U);
    fn size<const K: usize>(&self) -> [u8; K];
}

struct X(u32);
struct Y(u32);

impl Tr for X {
    fn tag<X: Clone>(&self, x: &X) -> X {
        x.clone()
    }
    fn convert<T: Copy, U: Copy>(&self, t: T, u: U) -> (T, U) {
        (t, u)
    }
    fn size<const K: usize>(&self) -> [u8; K] {
        [self.0 as u8; K]
    }
}

impl Tr for Y {
    fn tag<X: Clone>(&self, x: &X) -> X {
        x.clone()
    }
    fn convert<T: Copy, U: Copy>(&self, t: T, u: U) -> (T, U) {
        (t, u)
    }
    fn size<const K: usize>(&self) -> [u8; K] {
        [self.0 as u8; K]
    }
}

fn main() {

    let t = (X(1), Y(2));
    assert_eq!(t.tag(&42), (42, 42));
    assert_eq!(t.convert(1, "s"), ((1, "s"), (1, "s")));
    assert_eq!(t.size::<3>(), ([1, 1, 1], [2, 2, 2]));
}
