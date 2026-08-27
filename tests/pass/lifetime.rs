//! Traits with lifetime parameters: `'a` is lifted into the helper trait and
//! shared by all elements (shared track).

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<'a> {
    fn first(&self, s: &'a str) -> &'a str;
    fn len(&self, s: &'a str) -> usize;
}

struct X;
struct Y;

impl<'a> Tr<'a> for X {
    fn first(&self, s: &'a str) -> &'a str {
        &s[..1]
    }
    fn len(&self, s: &'a str) -> usize {
        s.len()
    }
}

impl<'a> Tr<'a> for Y {
    fn first(&self, s: &'a str) -> &'a str {
        &s[..2]
    }
    fn len(&self, s: &'a str) -> usize {
        s.len() * 2
    }
}

fn main() {

    let t = (X, Y);
    let s = String::from("hello");
    let (a, b) = t.first(&s);
    assert_eq!((a, b), ("h", "he"));
    assert_eq!(t.len(&s), (5, 10));
}
