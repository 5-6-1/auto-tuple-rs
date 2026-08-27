//! `use<..>` precise capturing in returns is not supported yet.

use auto_tuple::auto_tuple;

#[auto_tuple]
trait Tr<'a> {
    fn tokens(&self, s: &'a str) -> impl Iterator<Item = &'a str> + use<'a, Self>;
}

struct X;

impl<'a> Tr<'a> for X {
    fn tokens(&self, s: &'a str) -> impl Iterator<Item = &'a str> {
        vec![s].into_iter()
    }
}

fn main() {}
