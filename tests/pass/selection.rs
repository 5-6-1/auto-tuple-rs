//! Member families, default selection, exclusions and analysis interactions.
use auto_tuple::auto_tuple;

mod default_all {
    use super::*;

    #[auto_tuple(0..=2)]
    trait Read<T> {
        fn read(&self, _value: &T) -> T;
        const LIMIT: usize;
        type Output;
    }

    impl Read<u32> for u8 {
        fn read(&self, value: &u32) -> u32 {
            value + u32::from(*self)
        }
        const LIMIT: usize = 8;
        type Output = u8;
    }
    impl Read<u32> for u16 {
        fn read(&self, value: &u32) -> u32 {
            value + u32::from(*self)
        }
        const LIMIT: usize = 16;
        type Output = u16;
    }

    pub fn check() {
        assert_eq!((1u8, 2u16).read(&10), (11, 12));
        assert_eq!(<(u8, u16) as _ReadTuple2<u8, u16, u32>>::LIMIT, (8, 16));
        let _: <(u8, u16) as _ReadTuple2<u8, u16, u32>>::Output = (1u8, 2u16);
        assert_eq!(<(u8,) as _ReadTuple1<u8, u32>>::LIMIT, (8,));
        let _: <(u8,) as _ReadTuple1<u8, u32>>::Output = (1u8,);
        let _: <() as _ReadTuple0<u32>>::Output = ();
        <() as _ReadTuple0<u32>>::read(&(), &10);
        let () = <() as _ReadTuple0<u32>>::LIMIT;
    }
}

mod ref_selection {
    use super::*;

    #[auto_tuple(2..=2, [@all_ref_methods, MAX], -[reset], pub(crate), name = "Read")]
    trait Tr {
        fn read(&self) -> usize;
        fn reset(&mut self);
        fn boxed(self: Box<Self>);
        const MAX: usize = 12;
        type Output;
    }
    impl Tr for u8 {
        fn read(&self) -> usize {
            usize::from(*self)
        }
        fn reset(&mut self) {
            *self = 0;
        }
        fn boxed(self: Box<Self>) {}
        type Output = u8;
    }
    pub fn check() {
        assert_eq!((3u8, 4u8).read(), (3, 4));
        assert_eq!(<(u8, u8)>::MAX, (12, 12));
    }
}

mod independent_parameters {
    use super::*;

    // Neither the excluded method nor the unselected associated type may
    // force Shared; the selected call works with heterogeneous Tr arguments.
    #[auto_tuple(@all_methods, -depends, 2..=2)]
    trait Tr<T> {
        fn read(&self) -> usize;
        fn depends(_: &T) {}
        type Output: AsRef<T>;
    }
    impl Tr<u32> for u8 {
        fn read(&self) -> usize {
            1
        }
        type Output = Box<u32>;
    }
    impl Tr<String> for u16 {
        fn read(&self) -> usize {
            2
        }
        type Output = Box<String>;
    }
    pub fn check() {
        assert_eq!((0u8, 0u16).read(), (1, 2));
        assert_eq!(<(u8, u16) as _TrTuple2All<u8, u16, u32, String>>::read(&(0, 0)), (1, 2));
    }
}

mod exclusions {
    use super::*;

    #[auto_tuple(-boxed, -@all_types, 2..=2)]
    trait Tr {
        fn read(&self) -> usize {
            7
        }
        fn boxed(self: Box<Self>) {}
        const VALUE: usize = 9;
        type Output<T>;
    }
    impl Tr for u8 {
        type Output<T> = T;
    }

    // Explicitly selecting nothing must not restore the unsupported member.
    #[auto_tuple(@all, -@all, 2..=2)]
    trait Empty {
        fn boxed(self: Box<Self>) {}
    }
    impl Empty for u8 {}

    // A positive family with no matches is also an empty selection.
    #[auto_tuple(@all_types, 2..=2)]
    trait NoTypes {
        fn boxed(self: Box<Self>) {}
    }
    impl NoTypes for u8 {}

    pub fn check() {
        assert_eq!((1u8, 2u8).read(), (7, 7));
        assert_eq!(<(u8, u8)>::VALUE, (9, 9));
        fn empty<T: _EmptyTuple2<u8, u8>>() {}
        fn no_types<T: _NoTypesTuple2<u8, u8>>() {}
        empty::<(u8, u8)>();
        no_types::<(u8, u8)>();
    }
}

mod associated_items_choose_shared {
    use super::*;

    #[auto_tuple(2..=2)]
    trait Tr<T> {
        fn read(&self) -> usize {
            1
        }
        type Output: AsRef<T>;
    }
    impl Tr<u32> for u8 {
        type Output = Box<u32>;
    }
    impl Tr<u32> for u16 {
        type Output = Box<u32>;
    }

    pub fn check() {
        assert_eq!(<(u8, u16) as _TrTuple2<u8, u16, u32>>::read(&(0, 0)), (1, 1));
        let _: <(u8, u16) as _TrTuple2<u8, u16, u32>>::Output = (Box::new(1), Box::new(2));
    }
}

mod defaults_and_receivers {
    use super::*;

    #[auto_tuple(@all_default, @all_value_methods, @all_static_methods, -blocked, 2..=2)]
    trait Tr: Sized {
        fn read(&self) -> usize {
            1
        }
        fn take(self) -> usize;
        fn new() -> Self;
        fn blocked(self: Box<Self>) {}
        const DEFAULT: usize = 2;
        const REQUIRED: usize;
    }
    impl Tr for u8 {
        fn read(&self) -> usize {
            usize::from(*self)
        }
        fn take(self) -> usize {
            usize::from(self)
        }
        fn new() -> Self {
            3
        }
        const REQUIRED: usize = 4;
    }
    pub fn check() {
        assert_eq!(<(u8, u8)>::new(), (3, 3));
        assert_eq!((5u8, 6u8).read(), (5, 6));
        assert_eq!((7u8, 8u8).take(), (7, 8));
        assert_eq!(<(u8, u8)>::DEFAULT, (2, 2));
    }
}

fn main() {
    default_all::check();
    ref_selection::check();
    independent_parameters::check();
    exclusions::check();
    associated_items_choose_shared::check();
    defaults_and_receivers::check();
}
