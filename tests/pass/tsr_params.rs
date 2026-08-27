//! `Self`-shaped parameters are unpacked per element: `&Self`, `&mut Self`,
//! by-value `Self`, `Self::Output` and `&Self::Output`.

use auto_tuple::auto_tuple;

#[auto_tuple(MAX, Output, bump, swap, share, take_out, peek_out)]
trait Tr {
    const MAX: usize;
    type Output;

    fn bump(&self, other: &Self) -> usize;
    fn swap(&mut self, other: &mut Self);
    fn share(self, other: Self) -> usize;
    fn take_out(&self, x: Self::Output) -> usize;
    fn peek_out(&self, x: &Self::Output) -> usize;
}

struct X(usize);
struct Y(usize);

impl Tr for X {
    const MAX: usize = 10;
    type Output = usize;

    fn bump(&self, other: &Self) -> usize {
        self.0 + other.0
    }
    fn swap(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.0, &mut other.0);
    }
    fn share(self, other: Self) -> usize {
        self.0 + other.0
    }
    fn take_out(&self, x: Self::Output) -> usize {
        self.0 + x
    }
    fn peek_out(&self, x: &Self::Output) -> usize {
        self.0 + x
    }
}

impl Tr for Y {
    const MAX: usize = 20;
    type Output = usize;

    fn bump(&self, other: &Self) -> usize {
        self.0 * other.0
    }
    fn swap(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.0, &mut other.0);
    }
    fn share(self, other: Self) -> usize {
        self.0 + other.0
    }
    fn take_out(&self, x: Self::Output) -> usize {
        self.0 + x
    }
    fn peek_out(&self, x: &Self::Output) -> usize {
        self.0 + x
    }
}

fn main() {
    use crate::_TrTuple2;

    // &Self parameter: each element receives its own element of `other`.
    let t = (X(1), Y(2));
    let o = (X(3), Y(4));
    assert_eq!(t.bump(&o), (4, 8));

    // &mut Self parameter: independent mutable borrows of the two fields.
    let mut t = (X(1), Y(2));
    let mut o = (X(3), Y(4));
    t.swap(&mut o);
    assert_eq!((t.0.0, t.1.0), (3, 4));
    assert_eq!((o.0.0, o.1.0), (1, 2));

    // by-value Self parameter: the two fields are moved independently.
    let a = (X(1), Y(2));
    let b = (X(3), Y(4));
    assert_eq!(a.share(b), (4, 6));

    // Self::Output / &Self::Output parameters: unpacked from the tuple.
    assert_eq!(t.take_out((10, 20)), (13, 24));
    assert_eq!(t.peek_out(&(10, 20)), (13, 24));

    assert_eq!(<(X, Y)>::MAX, (10, 20));
    fn takes(_: <(X, Y) as _TrTuple2<X, Y>>::Output) {}
    takes((0usize, 0usize));
}
