//! Lifetime params with bounds (`'b: 'a`) force the shared track, which lifts
//! the original params and their bounds as-is.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<'a, 'b: 'a> {
    fn f(&self) -> usize;
    fn use_both(&self, x: &'a str, y: &'b str) -> &'a str;
}

struct X;
struct Y;

impl<'a, 'b: 'a> Tr<'a, 'b> for X {
    fn f(&self) -> usize {
        1
    }
    fn use_both(&self, x: &'a str, _y: &'b str) -> &'a str {
        x
    }
}

impl<'a, 'b: 'a> Tr<'a, 'b> for Y {
    fn f(&self) -> usize {
        2
    }
    fn use_both(&self, x: &'a str, _y: &'b str) -> &'a str {
        x
    }
}

fn main() {
    use crate::_TrTuple2;

    assert_eq!((X, Y).f(), (1, 2));
    let s = String::from("hi");
    assert_eq!((X, Y).use_both(&s, &s), ("hi", "hi"));
}
