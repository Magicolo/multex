use crate::lock::Mask;
use orn::*;
use std::{alloc::Layout, array::from_fn, mem::transmute};
use thiserror::*;

pub struct Key<L, G> {
    pub(crate) mask: L,
    pub(crate) taken: L,
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
    fn fold<S, E>(&self, state: S, fold: impl FnMut(S, T) -> Result<S, E>) -> Result<S, E>;
}

pub unsafe trait Get<'a, T: ?Sized> {
    type Item;
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item;
}

#[derive(Debug, Error)]
#[error("Invalid index '{index}'.")]
pub struct InvalidIndexError {
    pub index: usize,
}

impl<M: Mask, F: Fold<usize>> Key<M, F> {
    pub fn new(indices: F) -> Result<Self, InvalidIndexError> {
        let mask = indices.fold(M::new(), |mut mask, index| {
            if mask.add(index) {
                Ok(mask)
            } else {
                Err(InvalidIndexError { index })
            }
        })?;
        Ok(Self {
            mask,
            taken: M::new(),
            indices,
        })
    }
}

impl Fold<usize> for usize {
    fn fold<S, E>(&self, state: S, mut fold: impl FnMut(S, usize) -> Result<S, E>) -> Result<S, E> {
        fold(state, *self)
    }
}

impl<const I: usize> Fold<usize> for At<I> {
    fn fold<S, E>(&self, state: S, fold: impl FnMut(S, usize) -> Result<S, E>) -> Result<S, E> {
        I.fold(state, fold)
    }
}

impl<T, F: Fold<T>, const N: usize> Fold<T> for [F; N] {
    fn fold<S, E>(&self, mut state: S, mut fold: impl FnMut(S, T) -> Result<S, E>) -> Result<S, E> {
        for item in self.iter() {
            state = item.fold(state, &mut fold)?;
        }
        Ok(state)
    }
}

impl<T, F: Fold<T>> Fold<T> for [F] {
    fn fold<S, E>(&self, mut state: S, mut fold: impl FnMut(S, T) -> Result<S, E>) -> Result<S, E> {
        for item in self.iter() {
            state = item.fold(state, &mut fold)?;
        }
        Ok(state)
    }
}

impl<T, F: Fold<T> + ?Sized> Fold<T> for &F {
    fn fold<S, E>(&self, state: S, fold: impl FnMut(S, T) -> Result<S, E>) -> Result<S, E> {
        F::fold(self, state, fold)
    }
}

impl<T, F: Fold<T> + ?Sized> Fold<T> for &mut F {
    fn fold<S, E>(&self, state: S, fold: impl FnMut(S, T) -> Result<S, E>) -> Result<S, E> {
        F::fold(self, state, fold)
    }
}

unsafe impl<'a, T: 'a, const N: usize> Get<'a, [T; N]> for usize {
    type Item = Option<&'a mut T>;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut [T; N], mut filter: F) -> Self::Item {
        let index = *self;
        if index < N && filter(index) {
            Some(&mut *items.cast::<T>().add(index))
        } else {
            None
        }
    }
}

unsafe impl<'a, T: 'a> Get<'a, [T]> for usize {
    type Item = Option<&'a mut T>;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut [T], mut filter: F) -> Self::Item {
        let raw = transmute::<*mut [T], RawSlice<T>>(items);
        let index = *self;
        if index < raw.1 && filter(index) {
            Some(&mut *raw.0.add(index))
        } else {
            None
        }
    }
}

unsafe impl<'a, T: 'a, const N: usize> Get<'a, Box<[T; N]>> for usize {
    type Item = Option<&'a mut T>;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(
        &self,
        items: *mut Box<[T; N]>,
        filter: F,
    ) -> Self::Item {
        let raw = items.cast::<RawBox<[T; N]>>().read();
        <Self as Get<[T; N]>>::get(self, raw.0, filter)
    }
}

unsafe impl<'a, T: 'a> Get<'a, Box<[T]>> for usize {
    type Item = Option<&'a mut T>;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut Box<[T]>, filter: F) -> Self::Item {
        let raw = items.cast::<RawBox<[T]>>().read();
        <Self as Get<[T]>>::get(self, raw.0, filter)
    }
}

unsafe impl<'a, T: 'a> Get<'a, Vec<T>> for usize {
    type Item = Option<&'a mut T>;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut Vec<T>, filter: F) -> Self::Item {
        let raw = items.cast::<RawVec<T>>().read();
        let slice = transmute::<RawSlice<T>, *mut [T]>(RawSlice(raw.0, raw.2));
        <Self as Get<[T]>>::get(self, slice, filter)
    }
}

unsafe impl<'a, T: ?Sized + 'a> Get<'a, &'a mut T> for usize
where
    Self: Get<'a, T>,
{
    type Item = <Self as Get<'a, T>>::Item;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut &'a mut T, filter: F) -> Self::Item {
        <Self as Get<T>>::get(self, items.read(), filter)
    }
}

unsafe impl<'a, T: ?Sized + 'a> Get<'a, *mut T> for usize
where
    Self: Get<'a, T>,
{
    type Item = <Self as Get<'a, T>>::Item;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut *mut T, filter: F) -> Self::Item {
        <Self as Get<T>>::get(self, items.read(), filter)
    }
}

unsafe impl<'a, T: ?Sized + 'a, G: Get<'a, T>> Get<'a, T> for [G] {
    type Item = Vec<G::Item>;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut T, mut filter: F) -> Self::Item {
        let mut values = Vec::with_capacity(self.len());
        for get in self {
            values.push(get.get(items, &mut filter));
        }
        values
    }
}

unsafe impl<'a, T: ?Sized + 'a, G: Get<'a, T>, const N: usize> Get<'a, T> for [G; N] {
    type Item = [G::Item; N];

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut T, mut filter: F) -> Self::Item {
        from_fn(|index| self[index].get(items, &mut filter))
    }
}

unsafe impl<'a, T: ?Sized + 'a, G: Get<'a, T> + ?Sized> Get<'a, T> for &G {
    type Item = G::Item;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item {
        G::get(self, items, filter)
    }
}

unsafe impl<'a, T: ?Sized + 'a, G: Get<'a, T> + ?Sized> Get<'a, T> for &mut G {
    type Item = G::Item;

    #[inline]
    unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut T, filter: F) -> Self::Item {
        G::get(self, items, filter)
    }
}

macro_rules! tuples {
    ($n:expr, $or:ident $(, $tn:ident, $ti:ident, $i:tt)*) => {
        unsafe impl<'a, $($tn: 'a,)*> Get<'a, ($($tn,)*)> for usize {
            type Item = Option<$or<$(&'a mut $tn),*>>;

            #[inline]
            unsafe fn get<F: FnMut(usize) -> bool>(&self, _items: *mut ($($tn,)*), mut _filter: F) -> Self::Item {
                let index = *self;
                let mut _layout = Layout::new::<()>();
                let _offsets = ($({ let pair = _layout.extend(Layout::new::<$tn>()).unwrap(); _layout = pair.0; pair.1 },)*);
                match index {
                    $($i if _filter($i) => Some($or::$tn(unsafe { &mut *_items.cast::<u8>().add(_offsets.$i).cast::<$tn>() })),)*
                    _ => None,
                }
            }
        }

        unsafe impl<'a, T $(, $ti: Get<'a, T>)*> Get<'a, T> for ($($ti,)*) {
            type Item = ($($ti::Item,)*);

            #[inline]
            unsafe fn get<F: FnMut(usize) -> bool>(&self, _items: *mut T, mut _filter: F) -> Self::Item {
                #[allow(clippy::unused_unit)]
                ($(self.$i.get(_items, &mut _filter),)*)
            }
        }


        impl<T $(, $tn: Fold<T>)*> Fold<T> for ($($tn,)*) {
            #[inline]
            fn fold<S, E>(&self, mut _state: S, mut _fold: impl FnMut(S, T) -> Result<S, E>) -> Result<S, E> {
                $(_state = self.$i.fold(_state, &mut _fold)?;)*
                Ok(_state)
            }
        }
    };
}

macro_rules! at {
    ($ts:tt [$($t:ident, $i:tt),+]) => { $(at!(NEST $t, $i $ts);)+ };
    (NEST $t:ident, $i:tt [$($ts:ident),+]) => {
        unsafe impl<'a, $($ts: 'a),+> Get<'a, ($($ts,)+)> for At<$i> {
            type Item = Option<&'a mut $t>;

            #[inline]
            unsafe fn get<F: FnMut(usize) -> bool>(&self, items: *mut ($($ts,)+), mut filter: F) -> Self::Item {
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

tuples!(0, Or0);
tuples!(1, Or1, T0, I0, 0);
tuples!(2, Or2, T0, I0, 0, T1, I1, 1);
tuples!(3, Or3, T0, I0, 0, T1, I1, 1, T2, I2, 2);
tuples!(4, Or4, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3);
tuples!(5, Or5, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4);
tuples!(6, Or6, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5);
tuples!(7, Or7, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6);
tuples!(
    8, Or8, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7, 7
);
tuples!(
    9, Or9, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7, 7,
    T8, I8, 8
);
tuples!(
    10, Or10, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9
);
tuples!(
    11, Or11, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10
);
tuples!(
    12, Or12, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11
);
tuples!(
    13, Or13, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11, T12, I12, 12
);
tuples!(
    14, Or14, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11, T12, I12, 12, T13, I13, 13
);
tuples!(
    15, Or15, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
    7, T8, I8, 8, T9, I9, 9, T10, I10, 10, T11, I11, 11, T12, I12, 12, T13, I13, 13, T14, I14, 14
);
tuples!(
    16, Or16, T0, I0, 0, T1, I1, 1, T2, I2, 2, T3, I3, 3, T4, I4, 4, T5, I5, 5, T6, I6, 6, T7, I7,
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
