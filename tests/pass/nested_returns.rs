//! Nested `Self` in return types: `Box<Self>`, `Vec<Self>`, `Option<Self>`
//! and `&Self` (elided lifetime).

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn boxed(&self) -> Box<Self>;
    fn vec(&self) -> Vec<Self>
    where
        Self: Sized;
    fn opt(&self) -> Option<Self>
    where
        Self: Sized;
    fn r(&self) -> &Self;
}

struct X(usize);
struct Y(usize);

impl Tr for X {
    fn boxed(&self) -> Box<Self> {
        Box::new(X(self.0 + 1))
    }
    fn vec(&self) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![X(self.0), X(self.0 + 1)]
    }
    fn opt(&self) -> Option<Self>
    where
        Self: Sized,
    {
        (self.0 > 0).then(|| X(self.0))
    }
    fn r(&self) -> &Self {
        self
    }
}

impl Tr for Y {
    fn boxed(&self) -> Box<Self> {
        Box::new(Y(self.0 + 10))
    }
    fn vec(&self) -> Vec<Self>
    where
        Self: Sized,
    {
        vec![Y(self.0)]
    }
    fn opt(&self) -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
    fn r(&self) -> &Self {
        self
    }
}

fn main() {

    let t = (X(1), Y(2));

    let (bx, by) = t.boxed();
    assert_eq!((bx.0, by.0), (2, 12));

    let (vx, vy) = t.vec();
    assert_eq!(vx.len(), 2);
    assert_eq!(vy.len(), 1);

    let (ox, oy) = t.opt();
    assert!(ox.is_some() && oy.is_none());

    let (rx, ry) = t.r();
    assert_eq!(rx.0, 1);
    assert_eq!(ry.0, 2);
}
