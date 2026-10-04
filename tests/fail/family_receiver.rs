use auto_tuple::auto_tuple;

#[auto_tuple(@all_value_methods)]
trait Tr { fn boxed(self: Box<Self>); }

fn main() {}
