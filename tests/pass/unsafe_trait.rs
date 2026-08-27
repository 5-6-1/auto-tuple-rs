//! Unsafe trait and unsafe fn forwarding.

use auto_tuple::auto_tuple;

#[auto_tuple]
unsafe trait Tr {
    unsafe fn peek(&self) -> usize;
    fn safe(&self) -> usize;
}

struct X(usize);
struct Y(usize);

unsafe impl Tr for X {
    unsafe fn peek(&self) -> usize {
        self.0
    }
    fn safe(&self) -> usize {
        self.0
    }
}

unsafe impl Tr for Y {
    unsafe fn peek(&self) -> usize {
        self.0
    }
    fn safe(&self) -> usize {
        self.0
    }
}

fn main() {
    use crate::_TrTuple2;

    let t = (X(1), Y(2));
    // SAFETY: plain field reads.
    unsafe {
        assert_eq!(t.peek(), (1, 2));
    }
    assert_eq!(t.safe(), (1, 2));
}
