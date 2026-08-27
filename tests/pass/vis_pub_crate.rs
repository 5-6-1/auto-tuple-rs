//! `pub(crate)` visibility override: the helper trait stays crate-local even
//! when the original trait is `pub`, so internal machinery never leaks into
//! the public API.

mod inner {
    use auto_tuple::auto_tuple;

    #[auto_tuple(pub(crate))]
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
    // `pub(crate)` is reachable anywhere in this crate.
    use inner::_TrTuple2;

    let t = (inner::X(1), inner::Y(2));
    assert_eq!(t.get(), (1, 2));
    assert_eq!(<(inner::X, inner::Y)>::get(&t), (1, 2));
}
