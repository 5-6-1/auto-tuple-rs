//! Across modules the helper trait must be brought into scope, exactly like
//! any other trait's methods (`use std::io::Read` to call `.read()`). Within
//! the defining module no import is needed at all.

mod inner {
    use auto_tuple::auto_tuple;

    #[auto_tuple]
    pub trait Tr {
        fn get(&self) -> usize;
    }

    pub struct X(pub usize);
    pub struct Y(pub usize);

    impl Tr for X {
        fn get(&self) -> usize {
            self.0
        }
    }
    impl Tr for Y {
        fn get(&self) -> usize {
            self.0
        }
    }
}

fn main() {
    // Without the import the method is unresolvable; with it, it works.
    use inner::_TrTuple2;

    let t = (inner::X(1), inner::Y(2));
    assert_eq!(t.get(), (1, 2));
}
