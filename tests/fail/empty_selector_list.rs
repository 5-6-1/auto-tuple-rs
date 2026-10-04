use auto_tuple::auto_tuple;

#[auto_tuple(@all, -[])]
trait Tr { fn read(&self); }

fn main() {}
