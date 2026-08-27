//! RPIT (`impl Trait`) return types are tuple-ized element-wise.
//!
//! Each element returns its own opaque type, so the helper trait's return
//! becomes one `impl Trait` per element; the impl's hidden types are the
//! elements' own opaques.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn nums(&self) -> impl Iterator<Item = usize>;
    fn selfs(&self) -> impl Iterator<Item = Self>;
}

struct X;
struct Y;

impl Tr for X {
    fn nums(&self) -> impl Iterator<Item = usize> {
        vec![1, 2].into_iter()
    }
    fn selfs(&self) -> impl Iterator<Item = Self> {
        vec![X].into_iter()
    }
}

impl Tr for Y {
    fn nums(&self) -> impl Iterator<Item = usize> {
        vec![3, 4].into_iter()
    }
    fn selfs(&self) -> impl Iterator<Item = Self> {
        vec![Y].into_iter()
    }
}

fn main() {

    let t = (X, Y);

    // Plain RPIT: each element keeps its own opaque iterator.
    let (a, b) = t.nums();
    let na: Vec<usize> = a.collect();
    let nb: Vec<usize> = b.collect();
    assert_eq!((na, nb), (vec![1, 2], vec![3, 4]));

    // RPIT mentioning `Self`: element-wise `Item`.
    let (sa, sb) = t.selfs();
    assert_eq!(sa.count(), 1);
    assert_eq!(sb.count(), 1);
}
