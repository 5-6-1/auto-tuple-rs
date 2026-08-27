//! Auto traits cannot be tuple-ized.

use auto_tuple::auto_tuple;

#[auto_tuple]
auto trait Tr {}

fn main() {}
