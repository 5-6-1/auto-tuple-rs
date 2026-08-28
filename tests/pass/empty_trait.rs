//! An empty trait generates an empty helper trait + impl without issues.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr {}

struct X;
struct Y;

impl Tr for X {}
impl Tr for Y {}

fn main() {
    // Nothing to call; the point is that this compiles.
    let _t = (X, Y);
}
