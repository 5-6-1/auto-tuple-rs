//! An associated type bound mentioning `Self` is dropped from the helper
//! trait; each element's own `Out` already satisfies `Into<Self>`.

use auto_tuple::auto_tuple;

#[auto_tuple(Out, get)]
trait Tr
where
    Self: Sized,
{
    type Out: Into<Self>;
    fn get(&self) -> Self::Out;
}

struct X;
struct Y;

impl Tr for X {
    type Out = X;
    fn get(&self) -> X {
        X
    }
}

impl Tr for Y {
    type Out = Y;
    fn get(&self) -> Y {
        Y
    }
}

fn main() {
    use crate::_TrTuple2;

    let t = (X, Y);
    let (a, b) = t.get();
    let _: X = a.into();
    let _: Y = b.into();
}
