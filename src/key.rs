use crate::lock::{IndexError, Lock};
use std::{alloc::Layout, array::from_fn, mem::transmute};

pub struct Key<L, I>(L, I);
pub struct Index<const I: usize>;

#[repr(C)]
struct RawSlice<T: ?Sized>(*mut T, usize);
#[repr(C)]
struct RawVec<T: ?Sized>(*mut T, usize, usize);
#[repr(C)]
struct RawBox<T: ?Sized>(*mut T);

pub trait Fold<T> {
    fn fold<S>(&self, state: S, fold: impl FnMut(S, T) -> S) -> S;
}

pub unsafe trait At<T: ?Sized> {
    type Item<'a>
    where
        Self: 'a,
        T: 'a;
    unsafe fn at<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item<'a>
    where
        T: 'a;
}

impl<L: Lock, I: Fold<usize>> Key<L, I> {
    #[inline]
    pub fn new(indices: I) -> Result<Self, IndexError> {
        let mask = indices.fold(Ok(L::ZERO), |mask, index| L::add(mask?, index))?;
        Ok(Self(mask, indices))
    }
}

impl<L, I> Key<L, I> {
    #[inline]
    pub const fn mask(&self) -> &L {
        &self.0
    }

    #[inline]
    pub const fn indices(&self) -> &I {
        &self.1
    }
}

impl Fold<usize> for usize {
    fn fold<S>(&self, state: S, mut fold: impl FnMut(S, usize) -> S) -> S {
        fold(state, *self)
    }
}

impl<const I: usize> Fold<usize> for Index<I> {
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

unsafe impl<T, const N: usize> At<[T; N]> for usize {
    type Item<'a> = Option<&'a mut T> where T: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(
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

unsafe impl<T> At<[T]> for usize {
    type Item<'a> = Option<&'a mut T> where T: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(
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

unsafe impl<T, const N: usize> At<Box<[T; N]>> for usize {
    type Item<'a> = Option<&'a mut T>
    where
        Self: 'a,
        Box<[T; N]>: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut Box<[T; N]>,
        filter: F,
    ) -> Self::Item<'a>
    where
        Box<[T; N]>: 'a,
    {
        let raw = items.cast::<RawBox<[T; N]>>().read();
        <Self as At<[T; N]>>::at(self, raw.0, filter)
    }
}

unsafe impl<T> At<Box<[T]>> for usize {
    type Item<'a> = Option<&'a mut T>
    where
        Self: 'a,
        Box<[T]>: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut Box<[T]>,
        filter: F,
    ) -> Self::Item<'a>
    where
        Box<[T]>: 'a,
    {
        let raw = items.cast::<RawBox<[T]>>().read();
        <Self as At<[T]>>::at(self, raw.0, filter)
    }
}

unsafe impl<T> At<Vec<T>> for usize {
    type Item<'a> = Option<&'a mut T>
    where
        Self: 'a,
        Vec<T>: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut Vec<T>,
        filter: F,
    ) -> Self::Item<'a>
    where
        Vec<T>: 'a,
    {
        let raw = items.cast::<RawVec<T>>().read();
        let slice = transmute::<_, *mut [T]>(RawSlice(raw.0, raw.2));
        <Self as At<[T]>>::at(self, slice, filter)
    }
}

unsafe impl<'b, T> At<&'b mut T> for usize
where
    Self: At<T>,
{
    type Item<'a> = <Self as At<T>>::Item<'a>
    where
        Self: 'a,
        &'b mut T: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut &'b mut T,
        filter: F,
    ) -> Self::Item<'a>
    where
        &'b mut T: 'a,
    {
        <Self as At<T>>::at(self, items.read(), filter)
    }
}

unsafe impl<T> At<*mut T> for usize
where
    Self: At<T>,
{
    type Item<'a> = <Self as At<T>>::Item<'a>
    where
        Self: 'a,
        *mut T: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(
        &self,
        items: *mut *mut T,
        filter: F,
    ) -> Self::Item<'a>
    where
        *mut T: 'a,
    {
        <Self as At<T>>::at(self, items.read(), filter)
    }
}

unsafe impl<T, A: At<T>, const N: usize> At<T> for [A; N] {
    type Item<'a> = [A::Item<'a>; N]
        where
            Self: 'a,
            T: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, mut filter: F) -> Self::Item<'a>
    where
        T: 'a,
    {
        from_fn(|index| self[index].at(items, &mut filter))
    }
}

unsafe impl<T, A: At<T>> At<T> for &A {
    type Item<'a> = A::Item<'a>
        where
            Self: 'a,
            T: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item<'a>
    where
        Self: 'a,
        T: 'a,
    {
        A::at(self, items, filter)
    }
}

unsafe impl<T, A: At<T>> At<T> for &mut A {
    type Item<'a> = A::Item<'a>
        where
            Self: 'a,
            T: 'a;

    #[inline]
    unsafe fn at<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item<'a>
    where
        Self: 'a,
        T: 'a,
    {
        A::at(self, items, filter)
    }
}

macro_rules! tuples {
    ($n:expr, $one:ident $(, $tn:ident, $ti:ident, $i:tt)+) => {
        pub enum $one<$($tn),+> { $($tn($tn)),+ }

        unsafe impl<$($tn,)+> At<($($tn,)+)> for usize {
            type Item<'a> = Option<$one<$(&'a mut $tn),+>> where Self: 'a, ($($tn,)+): 'a;

            #[inline]
            unsafe fn at<'a, F: FnMut(usize) -> bool>(&self, items: *mut ($($tn,)+), mut filter: F) -> Self::Item<'a> where ($($tn,)+): 'a {
                let index = *self;
                let mut _layout = Layout::new::<()>();
                let offsets = ($({ let pair = _layout.extend(Layout::new::<$tn>()).unwrap(); _layout = pair.0; pair.1 },)+);
                match index {
                    $($i if filter($i) => Some($one::$tn(unsafe { &mut *items.cast::<u8>().add(offsets.$i).cast::<$tn>() })),)+
                    _ => None,
                }
            }
        }

        unsafe impl<T $(, $ti: At<T>)+> At<T> for ($($ti,)+) {
            type Item<'a> = ($($ti::Item<'a>,)+) where Self: 'a, T: 'a;

            #[inline]
            unsafe fn at<'a, F: FnMut(usize) -> bool>(&self, items: *mut T, mut filter: F) -> Self::Item<'a> where T: 'a {
                ($(self.$i.at(items, &mut filter),)+)
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
        unsafe impl<$($ts),+> At<($($ts,)+)> for Index<$i> {
            type Item<'a> = Option<&'a mut $t> where Self: 'a, ($($ts,)+): 'a;

            #[inline]
            unsafe fn at<'a, F: FnMut(usize) -> bool>(&self, items: *mut ($($ts,)+), mut filter: F) -> Self::Item<'a> where ($($ts,)+): 'a {
                if filter($i) {
                    let mut _layout = Layout::new::<()>();
                    let offsets = ($({ let pair = _layout.extend(Layout::new::<$ts>()).unwrap(); _layout = pair.0; pair.1 },)+);
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
