//! All track with arbitrary param shapes: each element instantiates `Tr` with
//! its own parameter group (multiple type params, lifetimes, const params).

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Multi<X, Y> {
    fn mf(&self) -> usize;
}

#[auto_tuple]
trait WithLifetime<'a> {
    fn lf(&self) -> usize;
}

#[auto_tuple]
trait WithConst<const N: usize> {
    fn cf(&self) -> usize;
}

struct X;
struct Y;

// Independent parameterizations per element.
impl Multi<u32, String> for X {
    fn mf(&self) -> usize {
        1
    }
}
impl Multi<String, u32> for Y {
    fn mf(&self) -> usize {
        2
    }
}

impl<'a> WithLifetime<'a> for X {
    fn lf(&self) -> usize {
        3
    }
}
impl<'a> WithLifetime<'a> for Y {
    fn lf(&self) -> usize {
        4
    }
}

impl WithConst<3> for X {
    fn cf(&self) -> usize {
        5
    }
}
impl WithConst<7> for Y {
    fn cf(&self) -> usize {
        6
    }
}

fn main() {
    use crate::{_MultiTuple2All, _WithConstTuple2All, _WithLifetimeTuple2All};

    assert_eq!((X, Y).mf(), (1, 2));
    assert_eq!((X, Y).lf(), (3, 4));
    assert_eq!((X, Y).cf(), (5, 6));
}
