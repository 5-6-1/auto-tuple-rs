//! Selected associated types whose bounds mention an original trait generic
//! param (`AsRef<T>`) or `Self` are dropped from the helper trait; the
//! element side is guaranteed by `A: Tr<T>`.

use auto_tuple::auto_tuple;

#[auto_tuple(Out, get)]
trait Tr<T> {
    type Out: AsRef<T>;
    fn get(&self) -> Self::Out;
}

struct X;
struct Y;

impl Tr<Vec<u8>> for X {
    type Out = Vec<u8>;
    fn get(&self) -> Vec<u8> {
        vec![1]
    }
}

impl Tr<Vec<u8>> for Y {
    type Out = Vec<u8>;
    fn get(&self) -> Vec<u8> {
        vec![2]
    }
}

fn main() {
    use crate::_TrTuple2;

    let t = (X, Y);
    assert_eq!(t.get(), (vec![1], vec![2]));
    fn typed(_: <(X, Y) as _TrTuple2<X, Y, Vec<u8>>>::Out) {}
    typed((vec![0u8], vec![0u8]));
}
