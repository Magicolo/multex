use crate::lock::Mask;
use std::{alloc::Layout, array::from_fn, mem::transmute};

pub struct Key<L, G> {
    pub(crate) mask: L,
    pub(crate) source: L,
    pub(crate) target: L,
    pub(crate) indices: G,
}

pub struct At<const I: usize>;

#[repr(C)]
struct RawSlice<T: ?Sized>(*mut T, usize);
#[repr(C)]
struct RawVec<T: ?Sized>(*mut T, usize, usize);
#[repr(C)]
struct RawBox<T: ?Sized>(*mut T);

pub trait Fold<T> {
    fn fold<S>(&self, state: S, fold: impl FnMut(S, T) -> S) -> S;
}

pub unsafe trait Get<T: ?Sized> {
    type Item<'a>
    where
        Self: 'a,
        T: 'a;
    unsafe fn get<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item<'a>
    where
        T: 'a;
}

impl<M: Mask, F: Fold<usize>> Key<M, F> {
    pub fn new(indices: F) -> Self {
        let mut mask = M::new();
        indices.fold(true, |_, index| mask.add(index));
        Self {
            mask,
            source: M::new(),
            target: M::new(),
            indices,
        }
    }
}

impl Fold<usize> for usize {
    fn fold<S>(&self, state: S, mut fold: impl FnMut(S, usize) -> S) -> S {
        fold(state, *self)
    }
}

impl<const I: usize> Fold<usize> for At<I> {
    fn fold<S>(&self, state: S, fold: impl FnMut(S, usize) -> S) -> S {
        I.fold(state, fold)
    }
}

impl<T, F: Fold<T>, const N: usize> Fold<T> for [F; N] {
    fn fold<S>(&self, mut state: S, mut fold: impl FnMut(S, T) -> S) -> S {
        for item in self.iter() {
            state = item.fold(state, &mut fold);
        }
        state
    }
}

impl<T, F: Fold<T>> Fold<T> for [F] {
    fn fold<S>(&self, mut state: S, mut fold: impl FnMut(S, T) -> S) -> S {
        for item in self.iter() {
            state = item.fold(state, &mut fold);
        }
        state
    }
}

impl<T, F: Fold<T>, const N: usize> Fold<T> for Box<[F; N]> {
    fn fold<S>(&self, mut state: S, mut fold: impl FnMut(S, T) -> S) -> S {
        for item in self.iter() {
            state = item.fold(state, &mut fold);
        }
        state
    }
}

impl<T, F: Fold<T>> Fold<T> for Box<[F]> {
    fn fold<S>(&self, mut state: S, mut fold: impl FnMut(S, T) -> S) -> S {
        for item in self.iter() {
            state = item.fold(state, &mut fold);
        }
        state
    }
}

impl<T, F: Fold<T>> Fold<T> for Vec<F> {
    fn fold<S>(&self, mut state: S, mut fold: impl FnMut(S, T) -> S) -> S {
        for item in self.iter() {
            state = item.fold(state, &mut fold);
        }
        state
    }
}

impl<T, F: Fold<T>> Fold<T> for &F {
    fn fold<S>(&self, state: S, fold: impl FnMut(S, T) -> S) -> S {
        F::fold(self, state, fold)
    }
}

impl<T, F: Fold<T>> Fold<T> for &mut F {
    fn fold<S>(&self, state: S, fold: impl FnMut(S, T) -> S) -> S {
        F::fold(self, state, fold)
    }
}

unsafe impl<T, const N: usize> Get<[T; N]> for usize {
    type Item<'a> = Option<&'a mut T> where T: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut [T; N],
        mut filter: F,
    ) -> Self::Item<'a>
    where
        [T; N]: 'a,
    {
        let index = *self;
        if index < N && filter(index) {
            Some(&mut *items.cast::<T>().add(index))
        } else {
            None
        }
    }
}

unsafe impl<T> Get<[T]> for usize {
    type Item<'a> = Option<&'a mut T> where T: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut [T],
        mut filter: F,
    ) -> Self::Item<'a>
    where
        [T]: 'a,
    {
        let raw = transmute::<_, RawSlice<T>>(items);
        let index = *self;
        if index < raw.1 && filter(index) {
            Some(&mut *raw.0.add(index))
        } else {
            None
        }
    }
}

unsafe impl<T, const N: usize> Get<Box<[T; N]>> for usize {
    type Item<'a> = Option<&'a mut T>
    where
        Self: 'a,
        Box<[T; N]>: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut Box<[T; N]>,
        filter: F,
    ) -> Self::Item<'a>
    where
        Box<[T; N]>: 'a,
    {
        let raw = items.cast::<RawBox<[T; N]>>().read();
        <Self as Get<[T; N]>>::get(self, raw.0, filter)
    }
}

unsafe impl<T> Get<Box<[T]>> for usize {
    type Item<'a> = Option<&'a mut T>
    where
        Self: 'a,
        Box<[T]>: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut Box<[T]>,
        filter: F,
    ) -> Self::Item<'a>
    where
        Box<[T]>: 'a,
    {
        let raw = items.cast::<RawBox<[T]>>().read();
        <Self as Get<[T]>>::get(self, raw.0, filter)
    }
}

unsafe impl<T> Get<Vec<T>> for usize {
    type Item<'a> = Option<&'a mut T>
    where
        Self: 'a,
        Vec<T>: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut Vec<T>,
        filter: F,
    ) -> Self::Item<'a>
    where
        Vec<T>: 'a,
    {
        let raw = items.cast::<RawVec<T>>().read();
        let slice = transmute::<_, *mut [T]>(RawSlice(raw.0, raw.2));
        <Self as Get<[T]>>::get(self, slice, filter)
    }
}

unsafe impl<'b, T> Get<&'b mut T> for usize
where
    Self: Get<T>,
{
    type Item<'a> = <Self as Get<T>>::Item<'a>
    where
        Self: 'a,
        &'b mut T: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut &'b mut T,
        filter: F,
    ) -> Self::Item<'a>
    where
        &'b mut T: 'a,
    {
        <Self as Get<T>>::get(self, items.read(), filter)
    }
}

unsafe impl<T> Get<*mut T> for usize
where
    Self: Get<T>,
{
    type Item<'a> = <Self as Get<T>>::Item<'a>
    where
        Self: 'a,
        *mut T: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut *mut T,
        filter: F,
    ) -> Self::Item<'a>
    where
        *mut T: 'a,
    {
        <Self as Get<T>>::get(self, items.read(), filter)
    }
}

unsafe impl<T, G: Get<T>, const N: usize> Get<T> for [G; N] {
    type Item<'a> = [G::Item<'a>; N]
        where
            Self: 'a,
            T: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut T,
        mut filter: F,
    ) -> Self::Item<'a>
    where
        T: 'a,
    {
        from_fn(|index| self[index].get(items, &mut filter))
    }
}

unsafe impl<T, G: Get<T>> Get<T> for &G {
    type Item<'a> = G::Item<'a>
        where
            Self: 'a,
            T: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item<'a>
    where
        Self: 'a,
        T: 'a,
    {
        G::get(self, items, filter)
    }
}

unsafe impl<T, G: Get<T>> Get<T> for &mut G {
    type Item<'a> = G::Item<'a>
        where
            Self: 'a,
            T: 'a;

    #[inline]
    unsafe fn get<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item<'a>
    where
        Self: 'a,
        T: 'a,
    {
        G::get(self, items, filter)
    }
}

macro_rules! tuples {
    ($n:expr, $one:ident $(, $tn:ident, $ti:ident, $i:tt)+) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $one<$($tn),+> { $($tn($tn)),+ }

        impl<$($tn),+> $one<$($tn),+> {
            #[inline]
            pub fn unify<T>(self) -> T where $($tn: Into<T>),+ {
                match self {
                    $(Self::$tn(item) => item.into(),)+
                }
            }

            #[inline]
            pub const fn as_ref(&self) -> $one<$(&$tn,)+> {
                match self {
                    $(Self::$tn(item) => $one::$tn(item),)+
                }
            }

            #[inline]
            pub fn as_mut(&mut self) -> $one<$(&mut $tn,)+> {
                match self {
                    $(Self::$tn(item) => $one::$tn(item),)+
                }
            }
        }

        unsafe impl<$($tn,)+> Get<($($tn,)+)> for usize {
            type Item<'a> = Option<$one<$(&'a mut $tn),+>> where Self: 'a, ($($tn,)+): 'a;

            #[inline]
            unsafe fn get<'a, F: FnMut(usize) -> bool>(&self, items: *mut ($($tn,)+), mut filter: F) -> Self::Item<'a> where ($($tn,)+): 'a {
                let index = *self;
                let mut _layout = Layout::new::<()>();
                let offsets = ($({ let pair = _layout.extend(Layout::new::<$tn>()).unwrap(); _layout = pair.0; pair.1 },)+);
                match index {
                    $($i if filter($i) => Some($one::$tn(unsafe { &mut *items.cast::<u8>().add(offsets.$i).cast::<$tn>() })),)+
                    _ => None,
                }
            }
        }

        unsafe impl<T $(, $ti: Get<T>)+> Get<T> for ($($ti,)+) {
            type Item<'a> = ($($ti::Item<'a>,)+) where Self: 'a, T: 'a;

            #[inline]
            unsafe fn get<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, mut filter: F) -> Self::Item<'a> where T: 'a {
                ($(self.$i.get(items, &mut filter),)+)
            }
        }


        impl<T $(, $tn: Fold<T>)+> Fold<T> for ($($tn,)+) {
            #[inline]
            fn fold<S>(&self, mut state: S, mut fold: impl FnMut(S, T) -> S) -> S {
                $(state = self.$i.fold(state, &mut fold);)+
                state
            }
        }
    };
}

macro_rules! at {
    ($ts:tt [$($t:ident, $i:tt),+]) => { $(at!(NEST $t, $i $ts);)+ };
    (NEST $t:ident, $i:tt [$($ts:ident),+]) => {
        unsafe impl<$($ts),+> Get<($($ts,)+)> for At<$i> {
            type Item<'a> = Option<&'a mut $t> where Self: 'a, ($($ts,)+): 'a;

            #[inline]
            unsafe fn get<'a, F: FnMut(usize) -> bool>(&self, items: *mut ($($ts,)+), mut filter: F) -> Self::Item<'a> where ($($ts,)+): 'a {
                let mut _layout = Layout::new::<()>();
                let offsets = ($({ let pair = _layout.extend(Layout::new::<$ts>()).unwrap(); _layout = pair.0; pair.1 },)+);
                if filter($i) {
                    Some(unsafe { &mut *items.cast::<u8>().add(offsets.$i).cast::<$t>() })
                } else {
                    None
                }
            }
        }
    };
}

tuples!(1, One1, T0, I0, 0);
tuples!(2, One2, T0, I0, 0, T1, I1, 1);
tuples!(3, One3, T0, I0, 0, T1, I1, 1, T2, I2, 2);
tuples!(4, One4, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3);
tuples!(5, One5, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4);
tuples!(6, One6, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5);
tuples!(7, One7, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6);
tuples!(
    8, One8, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7, 7
);
tuples!(
    9, One9, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8
);
tuples!(
    10, One10, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9
);
tuples!(
    11, One11, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10
);
tuples!(
    12, One12, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11
);
tuples!(
    13, One13, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11, T12, I12, 12
);
tuples!(
    14, One14, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11, T12, I12, 12, T13, I13, 13
);
tuples!(
    15, One15, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11, T12, I12, 12, T13, I13, 13, T14, I14, 14
);
tuples!(
    16, One16, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11, T12, I12, 12, T13, I13, 13, T14, I14, 14,
    T15, I15, 15
);

at!([T0] [T0, 0]);
at!([T0, T1] [T0, 0, T1, 1]);
at!([T0, T1, T2] [T0, 0, T1, 1, T2, 2]);
at!([T0, T1, T2, T3] [T0, 0, T1, 1, T2, 2, T3, 3]);
at!([T0, T1, T2, T3, T4] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4]);
at!([T0, T1, T2, T3, T4, T5] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5]);
at!([T0, T1, T2, T3, T4, T5, T6] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6]);
at!([T0, T1, T2, T3, T4, T5, T6, T7] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7]);
at!([T0, T1, T2, T3, T4, T5, T6, T7, T8] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8]);
at!([T0, T1, T2, T3, T4, T5, T6, T7, T8, T9] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9]);
at!([T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10]);
at!([T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11]);
at!([T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12]);
at!([T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12, T13, 13]);
at!([T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12, T13, 13, T14, 14]);
at!([T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15] [T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12, T13, 13, T14, 14, T15, 15]);
