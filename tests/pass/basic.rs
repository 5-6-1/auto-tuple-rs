//! Basic element-wise forwarding for a plain trait.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn zero() -> usize;
    fn pair(x: usize) -> (usize, usize);
    fn get(&self) -> usize;
    fn bump(&mut self) -> usize;
    fn take(self) -> usize;
}

struct X(u32);
struct Y(u32);

impl Tr for X {
    fn zero() -> usize {
        1
    }
    fn pair(x: usize) -> (usize, usize) {
        (x, x + 1)
    }
    fn get(&self) -> usize {
        self.0 as usize
    }
    fn bump(&mut self) -> usize {
        self.0 += 1;
        self.0 as usize
    }
    fn take(self) -> usize {
        self.0 as usize
    }
}

impl Tr for Y {
    fn zero() -> usize {
        10
    }
    fn pair(x: usize) -> (usize, usize) {
        (x * 2, x * 3)
    }
    fn get(&self) -> usize {
        self.0 as usize
    }
    fn bump(&mut self) -> usize {
        self.0 += 10;
        self.0 as usize
    }
    fn take(self) -> usize {
        self.0 as usize
    }
}

fn main() {

    // Associated function forwarding.
    assert_eq!(<(X, Y)>::zero(), (1, 10));

    // Argument forwarding.
    let v = 5;
    assert_eq!(<(X, Y)>::pair(v), ((5, 6), (10, 15)));

    // &self forwarding.
    let t = (X(3), Y(4));
    assert_eq!(t.get(), (3, 4));

    // &mut self forwarding.
    let mut t = (X(3), Y(4));
    assert_eq!(t.bump(), (4, 14));
    assert_eq!(t.get(), (4, 14));

    // by-value self forwarding.
    assert_eq!(t.take(), (4, 14));
}
