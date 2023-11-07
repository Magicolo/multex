mod key;
mod lock;
mod multex;
mod system;

pub use key::{
    At, Key, One1, One10, One11, One12, One13, One14, One15, One16, One2, One3, One4, One5, One6,
    One7, One8, One9,
};
pub use multex::{
    Guard, Multex, Multex16, Multex16N, Multex32, Multex32N, Multex64, Multex64N, Multex8,
    Multex8N, MultexN,
};

/*
    TODO:
    - Given a 'Guard<T>', allow to 'locK_with' and 'try_locK_with' additional resources.
        - 'lock_with' will first try to take the additional locks and if it fails, it will need to
        drop all of its locks and take them all at once on wake.
    - There's probably a way to implement 'LockAll' for 'Vec<T>' without allocations.
    - Add timeouts for `Multex` methods that might wait.
    - Compose keys with lock settings. This must modify the return type.
        - Key<_, At<0>> returns '&mut T'.
        - Key<_, Partial<At<0>>> returns 'Option<&mut T>'.
        - Key<_, Timeout<(At<0>, At<1>)> returns 'Option<(&mut T, &mut T)>'.
        - Is there a way to prevent redundance (ex: Partial<Partial<At<0>>>)?
        - Using types, reduces the size of the key.
    - System calls should take a 'u32' as the mask parameter.

    - How to get deadlock detection?
    - How to get re-entrant detection?
*/

#[test]
fn locks_different_indices() {
    let multex = Multex8::new([1u8, 2u8, 3u8, 4u8]);
    let mut key1 = Key::new([0]);
    let mut key2 = Key::new([1]);
    let mut guard1 = multex.lock_with(&mut key1, false);
    let mut guard2 = multex.lock_with(&mut key2, false);
    let Some(value1) = guard1[0].as_mut() else {
        panic!()
    };
    let Some(value2) = guard2[0].as_mut() else {
        panic!()
    };
    assert_eq!(**value1, 1u8);
    assert_eq!(**value2, 2u8);
}

#[test]
fn does_not_contend_on_out_of_bounds_indices() {
    let multex = Multex16::new([1u8, 2u8, 3u8, 4u8]);
    let mut key1 = Key::new([0, 4]);
    let mut key2 = Key::new([1, 4]);
    let _guard1 = multex.lock_with(&mut key1, false);
    let _guard2 = multex.lock_with(&mut key2, false);
}

#[test]
fn locks_all_without_panic() {
    Multex32::new(Vec::new()).lock().push(1);
}

#[test]
fn boba() {
    let multex = Multex8::new((1u8, 2u16));
    let mut key1 = Key::new(At::<1>);
    let mut key2 = Key::new((At::<1>, At::<0>));
    let mut key3 = Key::new([0, 1]);
    let mut key4 = Key::new((0, 1));
    let mut guard1 = multex.lock_with(&mut key1, false);
    let mut guard2 = multex.lock_with(&mut key2, false);
    let mut guard3 = multex.lock_with(&mut key3, false);
    let mut guard4 = multex.lock_with(&mut key4, false);
    **guard1.as_mut().as_mut().unwrap() += 1;
    **guard2.1.as_mut().unwrap() += 1;
    match guard3[0].as_mut().unwrap() {
        One2::T0(_1) => **_1 += 1,
        One2::T1(_2) => **_2 += 2,
    }
    match guard4.0.as_mut().unwrap() {
        One2::T0(_1) => **_1 += 1,
        One2::T1(_2) => **_2 += 2,
    }
}

fn fett() {
    struct Boba(usize, String, Vec<usize>);
    let mut boba1 = Boba(0, "".into(), vec![1, 2]);
    let mut vector1 = boba1.2.iter_mut();
    let multex1 = Multex64::new((
        &mut boba1.0,
        &mut boba1.1,
        vector1.next().unwrap(),
        vector1.next().unwrap(),
    ));
    let mut key1 = Key::new((1, 2));
    let mut guard1 = multex1.lock_with(&mut key1, false);
    if let (Some(One4::T1(a)), Some(One4::T2(b))) = &mut *guard1 {
        a.push('a');
        ***b += 1;
    }

    let mut boba2 = Boba(0, "".into(), vec![1, 2]);
    let mut vector2 = boba2.2.iter_mut();
    let multex2 = Multex8::new([
        One4::T0(&mut boba2.0),
        One4::T1(&mut boba2.1),
        One4::T2(vector2.next().unwrap()),
        One4::T3(vector2.next().unwrap()),
    ]);
    let mut key2 = Key::new((1, 2));
    let mut guard2 = multex2.lock_with(&mut key2, false);
    if let (Some(One4::T1(a)), Some(One4::T2(b))) = &mut *guard2 {
        a.push('a');
        **b += 1;
    }
}

mod dynamic {
    use orn::{Or1, Or4};
    use std::{
        borrow::{BorrowMut, Cow},
        collections::VecDeque,
        convert,
        marker::PhantomData,
        ops::{self, Deref, DerefMut},
    };

    pub struct Fett {
        a: Box<[usize]>,
    }

    pub struct Boba {
        a: usize,
        b: Fett,
        c: Vec<u8>,
        d: [i16; 2],
    }

    #[derive(Clone, Copy)]
    pub struct At<const I: usize>;
    #[derive(Clone, Copy)]
    pub struct Then<L, R>(L, R);

    pub trait Key<T> {
        type Value;
        const COUNT: usize;

        fn value(self, value: T) -> Self::Value;

        fn then<K>(self, key: K) -> Then<Self, K>
        where
            Self: Sized,
        {
            Then(self, key)
        }
    }

    impl<L, R> Then<L, R> {
        pub const fn then<K>(self, key: K) -> Then<Self, K> {
            Then(self, key)
        }
    }

    impl<const I: usize> At<I> {
        pub const fn then<K>(self, key: K) -> Then<Self, K> {
            Then(self, key)
        }
    }

    pub mod fett {
        #[derive(Clone, Copy)]
        pub struct A;
        #[derive(Clone, Copy)]
        pub enum Key {
            A,
        }
    }

    pub mod boba {
        use std::str::FromStr;

        #[derive(Clone, Copy)]
        pub struct A;
        #[derive(Clone, Copy)]
        pub struct B;
        #[derive(Clone, Copy)]
        pub struct C;
        #[derive(Clone, Copy)]
        pub struct D;
        #[derive(Clone, Copy)]
        pub enum Key {
            A,
            B,
            C,
            D,
        }

        impl TryFrom<&str> for Key {
            type Error = ();

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                match value {
                    "a" | "A" => Ok(Key::A),
                    "b" | "B" => Ok(Key::B),
                    "c" | "C" => Ok(Key::C),
                    "d" | "D" => Ok(Key::D),
                    _ => Err(()),
                }
            }
        }

        impl TryFrom<usize> for Key {
            type Error = ();

            fn try_from(value: usize) -> Result<Self, Self::Error> {
                match value {
                    0 => Ok(Key::A),
                    1 => Ok(Key::B),
                    2 => Ok(Key::C),
                    3 => Ok(Key::D),
                    _ => Err(()),
                }
            }
        }
    }

    macro_rules! boba {
        ($type: ident, $field: ident, $key: path, $index: tt, $value: ty) => {
            impl Key<$type> for $key {
                type Value = $value;
                const COUNT: usize = 1;

                fn value(self, $type { $field, .. }: $type) -> Self::Value {
                    $field
                }
            }

            impl<'a> Key<&'a $type> for $key {
                type Value = &'a $value;
                const COUNT: usize = 1;

                fn value(self, $type { $field, .. }: &'a $type) -> Self::Value {
                    $field
                }
            }

            impl<'a> Key<&'a mut $type> for $key {
                type Value = &'a mut $value;
                const COUNT: usize = 1;

                fn value(self, $type { $field, .. }: &'a mut $type) -> Self::Value {
                    $field
                }
            }

            impl Key<$type> for At<$index> {
                type Value = $value;
                const COUNT: usize = 1;

                fn value(self, $type { $field, .. }: $type) -> Self::Value {
                    $field
                }
            }

            impl<'a> Key<&'a $type> for At<$index> {
                type Value = &'a $value;
                const COUNT: usize = 1;

                fn value(self, $type { $field, .. }: &'a $type) -> Self::Value {
                    $field
                }
            }

            impl<'a> Key<&'a mut $type> for At<$index> {
                type Value = &'a mut $value;
                const COUNT: usize = 1;

                fn value(self, $type { $field, .. }: &'a mut $type) -> Self::Value {
                    $field
                }
            }

            impl $key {
                pub const fn then<K>(self, key: K) -> Then<Self, K> {
                    Then(self, key)
                }
            }
        };
    }

    boba!(Boba, a, boba::A, 0, usize);
    boba!(Boba, b, boba::B, 1, Fett);
    boba!(Boba, c, boba::C, 2, Vec<u8>);
    boba!(Boba, d, boba::D, 3, [i16; 2]);
    boba!(Fett, a, fett::A, 0, Box<[usize]>);

    impl<T, L: Key<T>, R: Key<L::Value>> Key<T> for Then<L, R> {
        type Value = R::Value;
        const COUNT: usize = L::COUNT.saturating_add(R::COUNT);

        fn value(self, value: T) -> Self::Value {
            self.1.value(self.0.value(value))
        }
    }

    impl<'a> Key<&'a Boba> for boba::Key {
        type Value = Or4<
            <boba::A as Key<&'a Boba>>::Value,
            <boba::B as Key<&'a Boba>>::Value,
            <boba::C as Key<&'a Boba>>::Value,
            <boba::D as Key<&'a Boba>>::Value,
        >;
        const COUNT: usize = <boba::A as Key<&'a Boba>>::COUNT
            + <boba::B as Key<&'a Boba>>::COUNT
            + <boba::C as Key<&'a Boba>>::COUNT
            + <boba::D as Key<&'a Boba>>::COUNT;

        fn value(self, value: &'a Boba) -> Self::Value {
            match self {
                boba::Key::A => Or4::T0(boba::A.value(value)),
                boba::Key::B => Or4::T1(boba::B.value(value)),
                boba::Key::C => Or4::T2(boba::C.value(value)),
                boba::Key::D => Or4::T3(boba::D.value(value)),
            }
        }
    }

    impl<'a> Key<&'a mut Boba> for boba::Key {
        type Value = Or4<
            <boba::A as Key<&'a mut Boba>>::Value,
            <boba::B as Key<&'a mut Boba>>::Value,
            <boba::C as Key<&'a mut Boba>>::Value,
            <boba::D as Key<&'a mut Boba>>::Value,
        >;
        const COUNT: usize = <boba::A as Key<&'a mut Boba>>::COUNT
            + <boba::B as Key<&'a mut Boba>>::COUNT
            + <boba::C as Key<&'a mut Boba>>::COUNT
            + <boba::D as Key<&'a mut Boba>>::COUNT;

        fn value(self, value: &'a mut Boba) -> Self::Value {
            match self {
                boba::Key::A => Or4::T0(boba::A.value(value)),
                boba::Key::B => Or4::T1(boba::B.value(value)),
                boba::Key::C => Or4::T2(boba::C.value(value)),
                boba::Key::D => Or4::T3(boba::D.value(value)),
            }
        }
    }

    impl Key<Boba> for boba::Key {
        type Value = Or4<
            <boba::A as Key<Boba>>::Value,
            <boba::B as Key<Boba>>::Value,
            <boba::C as Key<Boba>>::Value,
            <boba::D as Key<Boba>>::Value,
        >;
        const COUNT: usize = <boba::A as Key<Boba>>::COUNT
            + <boba::B as Key<Boba>>::COUNT
            + <boba::C as Key<Boba>>::COUNT
            + <boba::D as Key<Boba>>::COUNT;

        fn value(self, value: Boba) -> Self::Value {
            match self {
                boba::Key::A => Or4::T0(boba::A.value(value)),
                boba::Key::B => Or4::T1(boba::B.value(value)),
                boba::Key::C => Or4::T2(boba::C.value(value)),
                boba::Key::D => Or4::T3(boba::D.value(value)),
            }
        }
    }

    impl<'a> Key<&'a Fett> for fett::Key {
        type Value = Or1<<fett::A as Key<&'a Fett>>::Value>;
        const COUNT: usize = <fett::A as Key<&'a Fett>>::COUNT;

        fn value(self, value: &'a Fett) -> Self::Value {
            match self {
                fett::Key::A => Or1::T0(fett::A.value(value)),
            }
        }
    }

    macro_rules! slice {
        ($type: ty, $count: expr, [$($constant: ident)?]) => {
            impl<'a, T $(, const $constant: usize)?> Key<&'a $type> for usize {
                type Value = Option<&'a T>;
                const COUNT: usize = $count;

                fn value(self, value: &'a $type) -> Self::Value {
                    value.get(self)
                }
            }

            impl<'a, T $(, const $constant: usize)?> Key<&'a mut $type> for usize {
                type Value = Option<&'a mut T>;
                const COUNT: usize = $count;

                fn value(self, value: &'a mut $type) -> Self::Value {
                    value.get_mut(self)
                }
            }

            impl<'a, T, const I: usize $(, const $constant: usize)?> Key<&'a $type> for At<I> {
                type Value = Option<&'a T>;
                const COUNT: usize = $count;

                fn value(self, value: &'a $type) -> Self::Value {
                    value.get(I)
                }
            }

            impl<'a, T, const I: usize $(, const $constant: usize)?> Key<&'a mut $type> for At<I> {
                type Value = Option<&'a mut T>;
                const COUNT: usize = $count;

                fn value(self, value: &'a mut $type) -> Self::Value {
                    value.get_mut(I)
                }
            }
        };
    }

    // TODO: Implement for non-references (for sized types).
    // TODO: For dynamically sized types (such as Vec<T>), allow to declare a maximum size (Limit<T, const N: usize>(T))?
    slice!([T; N], N, [N]);
    slice!([T], usize::MAX, []);
    slice!(Vec<T>, usize::MAX, []);
    slice!(VecDeque<T>, usize::MAX, []);
    slice!(Box<[T]>, usize::MAX, []);
    slice!(Box<[T; N]>, N, [N]);

    impl<T, U, F: FnOnce(T) -> U> Key<T> for F {
        type Value = U;
        const COUNT: usize = 1;

        fn value(self, value: T) -> Self::Value {
            self(value)
        }
    }

    impl<T> Key<T> for () {
        type Value = T;
        const COUNT: usize = 0;

        fn value(self, value: T) -> Self::Value {
            value
        }
    }

    impl<T: Copy, K: Key<T>, const N: usize> Key<T> for [K; N] {
        type Value = [K::Value; N];
        const COUNT: usize = K::COUNT * N;

        fn value(self, value: T) -> Self::Value {
            self.map(|key| key.value(value))
        }
    }

    // TODO: Implement for other tuples.
    impl<T: Copy, K0: Key<T>, K1: Key<T>> Key<T> for (K0, K1) {
        type Value = (K0::Value, K1::Value);
        const COUNT: usize = K0::COUNT + K1::COUNT;

        fn value(self, value: T) -> Self::Value {
            (self.0.value(value), self.1.value(value))
        }
    }

    impl<T, K: Key<T>> Key<T> for Option<K> {
        type Value = Option<K::Value>;
        const COUNT: usize = K::COUNT;

        fn value(self, value: T) -> Self::Value {
            Some(self?.value(value))
        }
    }

    impl<T, E, K: Key<T>> Key<T> for Result<K, E> {
        type Value = Result<K::Value, E>;
        const COUNT: usize = K::COUNT;

        fn value(self, value: T) -> Self::Value {
            Ok(self?.value(value))
        }
    }

    #[test]
    fn boba() {
        // TODO: Keys should be allowed to be constructed as constant values where possible.
        // - Generate 'const fn then' for all relevant types (how to do for 'usize'?; do not use 'usize'? At(N::<0>), At(1)?).
        // - Rename Key::get.
        let boba = Boba {
            a: 1,
            b: Fett { a: Box::new([1]) },
            c: vec![1],
            d: [1, 2],
        };
        let _a = 1usize.value(&vec!['a']);
        let _a = ().value(&boba);
        let _a = boba::Key::try_from(1).value(&boba);
        let _a = boba::A.value(&boba);
        let _a = (boba::A, boba::C).value(&boba);
        let _a = boba::B.value(&boba);
        let _a = At::<0>.value(&boba);
        let _a = boba::C.then(1usize).value(&boba);
        let _a = boba::D.then(At::<2>).value(&boba);
        let _a = At::<3>.then(At::<2>).value(&boba);
        let _a = boba::B.then(fett::A).value(&boba);
        let _a = boba::B.then(fett::A).then(1usize).value(&boba);
        let _a = boba::B.then(fett::Key::A).value(&boba);
        let _a = boba::Key::try_from("a").value(&boba);
        let _a = At::<1>
            .then(fett::Key::A)
            .then(|Or1::T0(value)| value)
            .then(Deref::deref)
            .then(At::<1>)
            .value(&boba);
        let _a = boba::B
            .then(fett::Key::A)
            .then(|Or1::T0(value)| value)
            .then(AsRef::<[usize]>::as_ref)
            .then(at(1))
            .value(&boba);
        fn at<T>(index: usize) -> impl FnOnce(&[T]) -> &T {
            move |value| &value[index]
        }
        let _k = boba::B
            .then(fett::Key::A)
            .then(|Or1::T0(value)| value)
            .then(Clone::clone)
            .then(Into::into);
        let _a: Vec<usize> = _k.value(&boba);
        let _a: Vec<usize> = _k.value(&boba);
        let _a = match boba::Key::B.value(&boba) {
            Or4::T0(value) => Or4::T0(value),
            Or4::T1(value) => Or4::T1(fett::A.then(1).value(value)),
            Or4::T2(value) => Or4::T2(value),
            Or4::T3(value) => Or4::T3(value),
        };
    }
}

mod mule {
    use std::{
        cell::UnsafeCell,
        marker::PhantomData,
        ops::{Deref, DerefMut},
        ptr::addr_of_mut,
    };

    struct Mule<T>(UnsafeCell<T>);
    struct Root<T>(T);
    struct Node<P, C, B>(P, PhantomData<C>, B);
    struct Each<V>(V);
    struct Guard<'a, T>(T, &'a ());

    trait Key<T> {
        type Gather;
        type Value<'a>
        where
            T: 'a,
            Self: 'a;

        unsafe fn gather(&self, value: *mut T) -> Self::Gather;

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            T: 'a,
            Self: 'a;
    }

    trait Value {
        const COUNT: usize;
    }

    struct Get;
    struct At<const I: usize>;
    struct Then<L, R>(L, R);

    impl<T> Value for Root<T> {
        const COUNT: usize = 1;
    }

    impl<T: Value, U: Value, F> Value for Node<T, U, F> {
        const COUNT: usize = T::COUNT.saturating_add(U::COUNT);
    }

    impl<T, const N: usize> Value for Each<[T; N]> {
        const COUNT: usize = N;
    }

    impl<T> Value for Each<Vec<T>> {
        const COUNT: usize = usize::MAX;
    }

    impl<const I: usize> At<I> {
        pub const fn then<K>(self, key: K) -> Then<Self, K> {
            Then(self, key)
        }
    }

    impl<L, R> Then<L, R> {
        pub const fn then<K>(self, key: K) -> Then<Self, K> {
            Then(self, key)
        }
    }

    impl<T> Deref for Guard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for Guard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T> Root<T> {
        const fn birth<E, F: Fn(*mut T) -> E>(self, birth: F) -> Node<Self, E, F> {
            Node(self, PhantomData, birth)
        }
    }

    impl<T> Deref for Root<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for Root<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<P, C, B> Deref for Node<P, C, B> {
        type Target = P;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<P, C, B> DerefMut for Node<P, C, B> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T> Deref for Each<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for Each<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T, K: Key<T>> Key<T> for &K {
        type Gather = K::Gather;
        type Value<'a> = K::Value<'a> where T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut T) -> Self::Gather {
            K::gather(self, value)
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            T: 'a,
            Self: 'a,
        {
            K::value(value)
        }
    }

    impl<T, K: Key<T>> Key<T> for &mut K {
        type Gather = K::Gather;
        type Value<'a> = K::Value<'a> where T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut T) -> Self::Gather {
            K::gather(self, value)
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            T: 'a,
            Self: 'a,
        {
            K::value(value)
        }
    }

    impl<T> Key<*mut T> for Get {
        type Gather = *mut T;
        type Value<'a> = &'a mut T where T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut *mut T) -> Self::Gather {
            *value
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            T: 'a,
            Self: 'a,
        {
            &mut *value
        }
    }

    impl<T> Key<*const T> for Get {
        type Gather = *const T;
        type Value<'a> = &'a T where T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut *const T) -> Self::Gather {
            *value
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            T: 'a,
            Self: 'a,
        {
            &*value
        }
    }

    impl<'b, T> Key<&'b T> for Get {
        type Gather = &'b T;
        type Value<'a> = &'a T where &'b T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut &'b T) -> Self::Gather {
            *value
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            &'b T: 'a,
            Self: 'a,
        {
            value
        }
    }

    impl<'b, T> Key<&'b mut T> for Get {
        type Gather = &'b mut T;
        type Value<'a> = &'a mut T where &'b mut T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut &'b mut T) -> Self::Gather {
            *value
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            &'b mut T: 'a,
            Self: 'a,
        {
            value
        }
    }

    impl<T> Key<Root<T>> for Get {
        type Gather = *mut T;
        type Value<'a> = &'a mut T where Root<T>: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut Root<T>) -> Self::Gather {
            addr_of_mut!((*value).0)
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            Root<T>: 'a,
            Self: 'a,
        {
            &mut *value
        }
    }

    impl<T> Key<Each<T>> for Get
    where
        Self: Key<T>,
    {
        type Gather = <Self as Key<T>>::Gather;
        type Value<'a> = <Self as Key<T>>::Value<'a> where Each<T>: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut Each<T>) -> Self::Gather {
            <Self as Key<T>>::gather(self, addr_of_mut!((*value).0))
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            Each<T>: 'a,
            Self: 'a,
        {
            <Self as Key<T>>::value(value)
        }
    }

    impl<T, U, F> Key<Node<T, U, F>> for Get
    where
        Self: Key<T>,
    {
        type Gather = <Self as Key<T>>::Gather;
        type Value<'a> = <Self as Key<T>>::Value<'a> where Node<T, U, F>: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut Node<T, U, F>) -> Self::Gather {
            <Self as Key<T>>::gather(self, addr_of_mut!((*value).0))
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            Node<T, U, F>: 'a,
            Self: 'a,
        {
            <Self as Key<T>>::value(value)
        }
    }

    impl<T, L: Key<T>, R: Key<L::Gather>> Key<T> for Then<L, R> {
        type Gather = <R as Key<L::Gather>>::Gather;
        type Value<'a> = <R as Key<L::Gather>>::Value<'a> where T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut T) -> Self::Gather {
            self.1.gather(&mut self.0.gather(value))
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            T: 'a,
            Self: 'a,
        {
            <R as Key<L::Gather>>::value(value)
        }
    }

    impl<T, const I: usize> Key<*mut T> for At<I>
    where
        Self: Key<T>,
    {
        type Gather = <Self as Key<T>>::Gather;
        type Value<'a> = <Self as Key<T>>::Value<'a> where *mut T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut *mut T) -> Self::Gather {
            <Self as Key<T>>::gather(self, *value)
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            T: 'a,
            Self: 'a,
        {
            <Self as Key<T>>::value(value)
        }
    }

    impl<T, const I: usize> Key<Root<T>> for At<I>
    where
        Self: Key<T>,
    {
        type Gather = <Self as Key<T>>::Gather;
        type Value<'a> = <Self as Key<T>>::Value<'a> where Root<T>: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut Root<T>) -> Self::Gather {
            <Self as Key<T>>::gather(self, addr_of_mut!((*value).0))
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            Root<T>: 'a,
            Self: 'a,
        {
            <Self as Key<T>>::value(value)
        }
    }

    impl<T, U, F: Fn(<Get as Key<T>>::Gather) -> U, const N: usize> Key<Node<T, U, F>> for At<N>
    where
        Get: Key<T>,
        Self: Key<U>,
    {
        type Gather = <Self as Key<U>>::Gather;
        type Value<'a> = <Self as Key<U>>::Value<'a> where Node<T, U, F>: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut Node<T, U, F>) -> Self::Gather {
            let map = &mut *addr_of_mut!((*value).2);
            let value = <Get as Key<T>>::gather(&Get, addr_of_mut!((*value).0));
            <Self as Key<U>>::gather(self, &mut map(value))
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            Node<T, U, F>: 'a,
            Self: 'a,
        {
            <Self as Key<U>>::value(value)
        }
    }

    impl<T, const I: usize, const N: usize> Key<Each<[T; N]>> for At<I>
    where
        Self: Key<[T; N]>,
    {
        type Gather = <Self as Key<[T; N]>>::Gather;
        type Value<'a> = <Self as Key<[T; N]>>::Value<'a> where Each<[T; N]>: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut Each<[T; N]>) -> Self::Gather {
            <Self as Key<[T; N]>>::gather(self, addr_of_mut!((*value).0))
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            Each<[T; N]>: 'a,
            Self: 'a,
        {
            <Self as Key<[T; N]>>::value(value)
        }
    }

    impl<T, const I: usize, const N: usize> Key<[T; N]> for At<I> {
        type Gather = Option<*mut T>;
        type Value<'a> = Option<&'a mut T> where [T; N]: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut [T; N]) -> Self::Gather {
            if I < N {
                Some(value.cast::<T>().add(I))
            } else {
                None
            }
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            [T; N]: 'a,
            Self: 'a,
        {
            value.map(|value| &mut *value)
        }
    }

    impl<T1, T2, T3> Key<(T1, T2, T3)> for At<0>
    where
        Get: Key<T1>,
    {
        type Gather = *mut T1;
        type Value<'a> = <Get as Key<T1>>::Value<'a> where (T1, T2, T3): 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut (T1, T2, T3)) -> Self::Gather {
            addr_of_mut!((*value).0)
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            (T1, T2, T3): 'a,
            Self: 'a,
        {
            <Get as Key<T1>>::value(<Get as Key<T1>>::gather(&Get, value))
        }
    }

    impl<T1, T2, T3> Key<(T1, T2, T3)> for At<1>
    where
        Get: Key<T2>,
    {
        type Gather = *mut T2;
        type Value<'a> = <Get as Key<T2>>::Value<'a> where (T1, T2, T3): 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut (T1, T2, T3)) -> Self::Gather {
            addr_of_mut!((*value).1)
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            (T1, T2, T3): 'a,
            Self: 'a,
        {
            <Get as Key<T2>>::value(<Get as Key<T2>>::gather(&Get, value))
        }
    }

    impl<T1, T2, T3> Key<(T1, T2, T3)> for At<2>
    where
        Get: Key<T3>,
    {
        type Gather = *mut T3;
        type Value<'a> = <Get as Key<T3>>::Value<'a> where (T1, T2, T3): 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut (T1, T2, T3)) -> Self::Gather {
            addr_of_mut!((*value).2)
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            (T1, T2, T3): 'a,
            Self: 'a,
        {
            <Get as Key<T3>>::value(<Get as Key<T3>>::gather(&Get, value))
        }
    }

    impl<T, K1: Key<T>, K2: Key<T>> Key<T> for (K1, K2) {
        type Gather = (K1::Gather, K2::Gather);
        type Value<'a> = (K1::Value<'a>, K2::Value<'a>) where T: 'a, Self: 'a;

        unsafe fn gather(&self, value: *mut T) -> Self::Gather {
            (self.0.gather(value), self.1.gather(value))
        }

        unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
        where
            T: 'a,
            Self: 'a,
        {
            (K1::value(value.0), K2::value(value.1))
        }
    }

    unsafe impl<T: Sync> Sync for Mule<T> {}

    impl<T> Mule<T> {
        pub const fn new(value: T) -> Self {
            Self(UnsafeCell::new(value))
        }

        pub fn lock<K: Key<T>>(&self, key: K) -> Guard<'_, K::Value<'_>> {
            Guard(unsafe { K::value(key.gather(self.0.get())) }, &())
        }
    }

    const fn root<T>(value: T) -> Root<T> {
        Root(value)
    }

    macro_rules! child {
        ($parent:expr, $child:tt) => {
            addr_of_mut!((*$parent).$child)
        };
        ($parent:expr, $($child:tt),+) => {
            ($(child!($parent, $child)),+)
        };
    }

    macro_rules! node {
        ($($child:tt),+) => {
            |parent| unsafe { ($(addr_of_mut!((*parent).$child),)+) }
        };
    }

    fn test() {
        struct Boba {
            a: Option<Box<Boba>>,
            b: Vec<Boba>,
            c: [usize; 10],
        }
        const BOBA: Boba = Boba {
            a: None,
            b: vec![],
            c: [0; 10],
        };
        mod boba {
            use std::ops::{Index, IndexMut};

            struct Key {
                a: A,
                b: B,
                c: C,
            }
            struct A;
            struct B;
            struct C;

            // impl Index<usize> for B {
            //     type Output = Key;
            //     fn index(&self, index: usize) -> &Self::Output {
            //         &Key
            //     }
            // }
            // impl IndexMut<usize> for B {}
        }

        impl Key<Boba> for Get {
            type Gather = *mut Boba;
            type Value<'a> = &'a mut Boba where Boba: 'a, Self: 'a;

            unsafe fn gather(&self, value: *mut Boba) -> Self::Gather {
                value
            }

            unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
            where
                Boba: 'a,
                Self: 'a,
            {
                &mut *value
            }
        }

        impl Key<Boba> for At<0> {
            type Gather = *mut Option<Box<Boba>>;
            type Value<'a> = &'a mut Option<Box<Boba>> where Boba: 'a, Self: 'a;

            unsafe fn gather(&self, value: *mut Boba) -> Self::Gather {
                addr_of_mut!((*value).a)
            }

            unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
            where
                Boba: 'a,
                Self: 'a,
            {
                &mut *value
            }
        }

        impl Key<Boba> for At<1> {
            type Gather = *mut Vec<Boba>;
            type Value<'a> = &'a mut Vec<Boba> where Boba: 'a, Self: 'a;

            unsafe fn gather(&self, value: *mut Boba) -> Self::Gather {
                addr_of_mut!((*value).b)
            }

            unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
            where
                Boba: 'a,
                Self: 'a,
            {
                &mut *value
            }
        }

        impl Key<Boba> for At<2> {
            type Gather = *mut [usize; 10];
            type Value<'a> = &'a mut [usize; 10] where Boba: 'a, Self: 'a;

            unsafe fn gather(&self, value: *mut Boba) -> Self::Gather {
                addr_of_mut!((*value).c)
            }

            unsafe fn value<'a>(value: Self::Gather) -> Self::Value<'a>
            where
                Boba: 'a,
                Self: 'a,
            {
                &mut *value
            }
        }

        // TODO: How to deal with recursive values?
        /*
            value!(BOBA); // Locks only the root.
            value!(BOBA { a, b, c }); // Locks the root and children.
            value!(BOBA { a, b, c[..] }); // Locks the root, children and all values in 'c'.
            value!(BOBA { a, b, c[..10] }); // Locks the root, children and the 10 first values in 'c'.
            value!(BOBA { a, b: BOBA { a, b, c }, c });
            key!(Boba::a) // Locks the 'a' bit.
            key!(Boba) // Locks all bits.
            key!(Boba::b.b)
            key!(Boba::b.c[10]) // Locks the 'b.c' at index 10 bit.
            let index = 100;
            key!(Boba::b.c[index]) // Locks the 'b.c' at 'index' bit.

            #[derive(Value)]
            struct Boba;
            Boba::KEY.a;
            Boba::KEY.a.b;
            Boba::KEY.b.c[10];
        */
        let m = Mule::new(BOBA);
        let g = m.lock(Get);
        // let m = Mule::new((1u8, 2u16, 3u32));
        let g = m.lock(At::<1>);

        // root(BOBA).birth(node!(a, b, c));
        // root(BOBA).birth((node!(a), node!(b).birth(node!(a, b, c)), node!(c)));

        let mule = Mule::new(root(BOBA).birth(|boba| unsafe {
            (
                child!(boba, a),
                root(BOBA).birth(|boba| child!(boba, a, b, c)),
                child!(boba, c),
            )
        }));
        let _item = mule.lock(Get);
        let _item = mule.lock(At::<0>);
        let _item = mule.lock(At::<1>);
        let _item = mule.lock(At::<2>);
        let _item = mule.lock(At::<1>.then(Get));
        let _item = mule.lock((At::<2>, At::<1>));
        let _item = mule.lock(At::<2>.then(At::<2>));
        let _item = mule.lock(At::<1>);
        let mut _item = mule.lock(At::<1>.then(At::<2>).then(At::<0>));
        if let Some(_item) = _item.as_mut() {
            **_item += 1;
        }
    }
}

mod meta {
    pub enum Access {
        Private,
        Public { r#in: Option<In> },
    }
    pub enum In {
        Super,
        Crate,
        Ancestor { name: &'static str },
    }
    pub struct Module {
        pub access: Access,
        pub attributes: &'static [Attribute],
        pub modules: &'static [Module],
        pub structures: &'static [Structure],
        pub enumerations: &'static [Enumeration],
        pub unions: &'static [Union],
        pub traits: &'static [Trait],
        pub functions: &'static [Function],
    }
    pub struct Implementation {
        pub attributes: &'static [Attribute],
        pub r#type: &'static str,
        pub r#trait: Option<&'static str>,
        pub constants: &'static [Constant],
        pub statics: &'static [Static],
        pub lifetimes: &'static [Lifetime],
        pub generics: &'static [Generic],
        pub functions: &'static [Function],
    }
    pub struct Field {
        pub access: Access,
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub index: usize,
        pub r#type: &'static str,
    }
    pub struct Parameter {
        pub name: Option<&'static str>,
        pub attributes: &'static [Attribute],
        pub r#type: &'static str,
    }
    pub struct Lifetime {
        pub name: &'static str,
        pub attributes: &'static [Attribute],
        pub constraints: &'static [Constraint],
    }
    pub enum Primitive {
        Char,
        Bool,
        F32,
        F64,
        U8,
        U16,
        U32,
        U64,
        Usize,
        U128,
        I8,
        I16,
        I32,
        I64,
        Isize,
        I128,
    }
    pub struct Generic {
        pub name: &'static str,
        pub constant: Option<Primitive>,
        pub attributes: &'static [Attribute],
        pub constraints: &'static [Constraint],
    }
    pub struct Signature {
        pub lifetimes: &'static [Lifetime],
        pub generics: &'static [Generic],
        pub parameters: &'static [Parameter],
        pub r#return: &'static str,
    }
    pub struct Function {
        pub access: Access,
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub signature: Signature,
    }
    pub struct Constant {
        pub access: Access,
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub r#type: &'static str,
    }
    pub struct Static {
        pub access: Access,
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub mutable: bool,
        pub r#type: &'static str,
    }
    pub enum Attribute {
        Derive(&'static str),
    }
    pub enum Constraint {
        Lifetime(&'static str),
        Trait(&'static str),
    }

    pub struct Item {
        pub access: Access,
        pub attributes: &'static [Attribute],
        pub index: usize,
        pub r#type: &'static str,
    }
    pub enum Body {
        Unit,
        Tuple { items: &'static [Item] },
        Map { fields: &'static [Field] },
    }
    pub struct Structure {
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub module: &'static str,
        pub lifetimes: &'static [Lifetime],
        pub generics: &'static [Generic],
        pub body: Body,
    }
    pub struct Variant {
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub body: Body,
    }
    pub struct Enumeration {
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub module: &'static str,
        pub lifetimes: &'static [Lifetime],
        pub generics: &'static [Generic],
        pub variants: &'static [Variant],
    }
    pub struct Union {
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub module: &'static str,
        pub lifetimes: &'static [Lifetime],
        pub generics: &'static [Generic],
        pub fields: &'static [Field],
    }
    pub struct Associate {
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub lifetimes: &'static [Lifetime],
        pub generics: &'static [Generic],
        pub constraints: &'static [Constraint],
    }
    pub struct Definition {
        pub attributes: &'static [Attribute],
        pub name: &'static str,
        pub signature: Signature,
    }
    pub struct Base {
        pub attributes: &'static [Attribute],
        pub name: &'static str,
    }
    pub struct Trait {
        pub access: Access,
        pub name: &'static str,
        pub attributes: &'static [Attribute],
        pub bases: &'static [Base],
        pub lifetimes: &'static [Lifetime],
        pub generics: &'static [Generic],
        pub associates: &'static [Associate],
        pub definitions: &'static [Definition],
    }

    pub enum Data {
        Primitive(Primitive),
        Structure(Structure),
        Enumeration(Enumeration),
        Union(Union),
    }

    pub trait Meta<T> {
        const META: T;
    }

    #[derive(Clone, Copy)]
    struct Karl<'a, T: 'a, const N: usize> {
        a: &'a [T; N],
    }

    impl Meta<Primitive> for usize {
        const META: Primitive = Primitive::Usize;
    }

    impl Meta<Data> for usize {
        const META: Data = Data::Primitive(<Self as Meta<Primitive>>::META);
    }

    impl<'a, T, const N: usize> Meta<Data> for Karl<'a, T, N> {
        const META: Data = Data::Structure(<Self as Meta<Structure>>::META);
    }

    impl<'a, T, const N: usize> Meta<Structure> for Karl<'a, T, N> {
        const META: Structure = Structure {
            attributes: &[Attribute::Derive("Clone"), Attribute::Derive("Copy")],
            name: "Karl",
            module: module_path!(),
            lifetimes: &[Lifetime {
                attributes: &[],
                constraints: &[],
                name: "a",
            }],
            generics: &[
                Generic {
                    attributes: &[],
                    constant: None,
                    constraints: &[Constraint::Lifetime("a")],
                    name: "T",
                },
                Generic {
                    attributes: &[],
                    constant: Some(Primitive::Usize),
                    constraints: &[],
                    name: "N",
                },
            ],
            body: Body::Map {
                fields: &[Field {
                    access: Access::Private,
                    attributes: &[],
                    name: "a",
                    index: 0,
                    r#type: "&'a [T; N]",
                }],
            },
        };
    }
}
