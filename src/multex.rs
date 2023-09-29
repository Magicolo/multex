use crate::{
    key::{Get, Key},
    lock::{Lock, LockAll},
};
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

pub struct Multex<T: ?Sized, L: Lock = usize> {
    pub(crate) state: L::State,
    pub(crate) value: UnsafeCell<T>,
}
pub type MultexN<T, const N: usize> = Multex<T, [usize; N]>;
pub type Multex8<T> = Multex<T, u8>;
pub type Multex8N<T, const N: usize> = Multex<T, [u8; N]>;
pub type Multex16<T> = Multex<T, u16>;
pub type Multex16N<T, const N: usize> = Multex<T, [u16; N]>;
pub type Multex32<T> = Multex<T, u32>;
pub type Multex32N<T, const N: usize> = Multex<T, [u32; N]>;
pub type Multex64<T> = Multex<T, u64>;
pub type Multex64N<T, const N: usize> = Multex<T, [u64; N]>;

pub struct Guard<'a, T, L: Lock>(T, Inner<'a, L>);
/// [`Inner`] should be kept separate from [`Guard`] such that its [`Drop`] implementation is called even if
/// a panic occurs when the value `T` is produced.
struct Inner<'a, L: Lock>(&'a L::State, Borrow<'a, L>);
enum Borrow<'a, T> {
    Own(T),
    Mut(&'a mut T),
}

unsafe impl<T: Sync, L: Lock> Sync for Multex<T, L> {}

impl<T> Deref for Borrow<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Borrow::Own(value) => value,
            Borrow::Mut(value) => value,
        }
    }
}

impl<T> DerefMut for Borrow<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Borrow::Own(value) => value,
            Borrow::Mut(value) => value,
        }
    }
}

impl<'a, T, L: Lock> Guard<'a, T, L> {
    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(guard: Self, map: F) -> Guard<'a, U, L> {
        Guard(map(guard.0), guard.1)
    }

    #[inline]
    pub fn mask(&self) -> &L {
        &self.1 .1
    }
}

impl<T, L: Lock> Deref for Guard<'_, T, L> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, L: Lock> DerefMut for Guard<'_, T, L> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, L: Lock> AsRef<T> for Guard<'_, T, L> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T, L: Lock> AsMut<T> for Guard<'_, T, L> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<L: Lock> Drop for Inner<'_, L> {
    #[inline]
    fn drop(&mut self) {
        self.1.unlock(self.0, true);
        self.1.clear();
    }
}

impl<T, L: Lock> Multex<T, L> {
    #[inline]
    pub const fn new(values: T) -> Self {
        Self {
            state: L::NEW,
            value: UnsafeCell::new(values),
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized, L: LockAll> Multex<T, L> {
    #[inline]
    pub fn lock(&self) -> Guard<'_, &mut T, L> {
        let mut mask = L::ALL;
        if L::ALL.lock(&self.state, &mut mask, false, true) {
            unsafe { self.guard(mask) }
        } else {
            unreachable!()
        }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<Guard<&mut T, L>> {
        let mut mask = L::ALL;
        if L::ALL.lock(&self.state, &mut mask, false, false) {
            Some(unsafe { self.guard(mask) })
        } else {
            None
        }
    }

    /// Forcefully unlocks all the bits. A normal usage of a [`Multex`] normally doesn't require to unlock manually
    /// since the [`Guard`] already does it automatically. This method is mainly meant to be used when [`std::mem::forget(guard)`] is
    /// used.
    ///
    /// # Safety
    /// This method is marked as `unsafe` because it may unlock bits that are still locked by a [`Guard`]. A wrong usage of unlock will
    /// allow multiple concurrent mutable references to exist, thus causing undefined behavior.
    #[inline]
    pub unsafe fn unlock(&self) {
        L::ALL.unlock(&self.state, true);
    }

    #[inline]
    pub fn is_locked(&self, partial: bool) -> bool {
        L::ALL.is_locked(&self.state, partial)
    }

    #[inline]
    unsafe fn guard(&self, mask: L) -> Guard<&mut T, L> {
        let inner = Inner(&self.state, Borrow::Own(mask));
        Guard(unsafe { &mut *self.value.get() }, inner)
    }
}

impl<T: ?Sized, L: Lock> Multex<T, L> {
    #[inline]
    pub const fn as_ptr(&self) -> *const T {
        self.value.get()
    }

    #[inline]
    pub const fn as_mut_ptr(&self) -> *mut T {
        self.value.get()
    }

    #[inline]
    pub fn lock_with<'a, G: Get<T>>(
        &'a self,
        key: &'a mut Key<L, G>,
        partial: bool,
    ) -> Guard<'a, G::Item<'a>, L> {
        if key.mask.lock(&self.state, &mut key.source, partial, true) {
            unsafe { self.guard_with(key) }
        } else {
            unreachable!()
        }
    }

    #[inline]
    pub fn try_lock_with<'a, G: Get<T>>(
        &'a self,
        key: &'a mut Key<L, G>,
        partial: bool,
    ) -> Option<Guard<'a, G::Item<'a>, L>> {
        if key.mask.lock(&self.state, &mut key.source, partial, false) {
            Some(unsafe { self.guard_with(key) })
        } else {
            None
        }
    }

    /// Forcefully unlocks the bits contained in the provided `mask`. A normal usage of a [`Multex`] normally doesn't require to unlock
    /// manually since the [`Guard`] already does it automatically. This method is mainly meant to be used when
    /// [`std::mem::forget(guard)`] is used.
    ///
    /// # Safety
    /// This method is marked as `unsafe` because it may unlock bits that are still locked by a [`Guard`]. A wrong usage of unlock will
    /// allow multiple concurrent mutable references to exist, thus causing undefined behavior.
    #[inline]
    pub unsafe fn unlock_with<G: Get<T>>(&self, mask: &L) {
        mask.unlock(&self.state, true);
    }

    #[inline]
    pub fn is_locked_with<G: Get<T>>(&self, key: &Key<L, G>, partial: bool) -> bool {
        key.mask.is_locked(&self.state, partial)
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    #[inline]
    pub fn get_mut_with<G: Get<T>>(&mut self, key: &mut Key<L, G>) -> G::Item<'_> {
        key.source.clear();
        let value = self.value.get_mut();
        unsafe { key.indices.get(value, |index| key.source.add(index)) }
    }

    #[inline]
    unsafe fn guard_with<'a, G: Get<T>>(
        &'a self,
        key: &'a mut Key<L, G>,
    ) -> Guard<'a, G::Item<'a>, L> {
        let item = key.indices.get(self.value.get(), |index| {
            key.source.remove(index) && key.target.add(index)
        });
        // Unlock bits that were not used in the key.
        key.source.unlock(&self.state, true);
        Guard(item, Inner(&self.state, Borrow::Mut(&mut key.target)))
    }
}
