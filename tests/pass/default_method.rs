//! Default methods and lifetime elision in return types.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {
    fn base(&self) -> usize;
    fn derived(&self) -> usize {
        self.base() + 100
    }
    fn name(&self) -> &str;
}

struct X;
struct Y;

impl Tr for X {
    fn base(&self) -> usize {
        1
    }
    fn name(&self) -> &str {
        "x"
    }
}

impl Tr for Y {
    fn base(&self) -> usize {
        2
    }
    fn name(&self) -> &str {
        "y"
    }
}

fn main() {

    let t = (X, Y);

    // Default body is inherited through the element's own default impl.
    assert_eq!(t.derived(), (101, 102));

    // Elided lifetimes in `&str` returns tupleize fine.
    assert_eq!(t.name(), ("x", "y"));
}
