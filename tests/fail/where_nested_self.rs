//! `Self` inside a where-clause bound in non-subject position is rejected.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn f(&self)
    where
        Vec<Self>: Clone;
}

struct X;

impl Tr for X {
    fn f(&self)
    where
        Vec<Self>: Clone,
    {
    }
}

fn main() {}
