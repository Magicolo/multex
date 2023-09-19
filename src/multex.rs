use crate::{
    key::{At, Key},
    lock::Lock,
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
struct Inner<'a, L: Lock>(&'a L::State, L);

unsafe impl<T: Sync, L: Lock> Sync for Multex<T, L> {}

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

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, L: Lock> DerefMut for Guard<'_, T, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<L: Lock> Drop for Inner<'_, L> {
    fn drop(&mut self) {
        self.1.unlock(self.0, true);
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
    pub fn lock(&self) -> Guard<'_, &mut T, L> {
        match L::MAX.lock(&self.state, false, true) {
            Some(mask) => unsafe { self.guard(mask) },
            None => unreachable!(),
        }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<Guard<&mut T, L>> {
        let mask = L::MAX.lock(&self.state, false, false)?;
        Some(unsafe { self.guard(mask) })
    }

    #[inline]
    pub fn lock_with<A: At<T>>(&self, key: &Key<L, A>, partial: bool) -> Guard<'_, A::Item<'_>, L> {
        match key.mask().lock(&self.state, partial, true) {
            Some(mask) => unsafe { self.guard_with(mask, key) },
            None => unreachable!(),
        }
    }

    #[inline]
    pub fn try_lock_with<A: At<T>>(
        &self,
        key: &Key<L, A>,
        partial: bool,
    ) -> Option<Guard<'_, A::Item<'_>, L>> {
        let mask = key.mask().lock(&self.state, partial, false)?;
        Some(unsafe { self.guard_with(mask, key) })
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
        L::MAX.unlock(&self.state, true);
    }

    /// Forcefully unlocks the bits contained in the provided `mask`. A normal usage of a [`Multex`] normally doesn't require to unlock
    /// manually since the [`Guard`] already does it automatically. This method is mainly meant to be used when
    /// [`std::mem::forget(guard)`] is used.
    ///
    /// # Safety
    /// This method is marked as `unsafe` because it may unlock bits that are still locked by a [`Guard`]. A wrong usage of unlock will
    /// allow multiple concurrent mutable references to exist, thus causing undefined behavior.
    #[inline]
    pub unsafe fn unlock_with<A: At<T>>(&self, mask: &L) {
        mask.unlock(&self.state, true);
    }

    #[inline]
    pub fn is_locked(&self, partial: bool) -> bool {
        L::MAX.is_locked(&self.state, partial)
    }

    #[inline]
    pub fn is_locked_with<A: At<T>>(&self, key: &Key<L, A>, partial: bool) -> bool {
        key.mask().is_locked(&self.state, partial)
    }

    #[inline]
    pub fn get_mut<A: At<T>>(&mut self, key: &Key<L, A>) -> A::Item<'_> {
        unsafe { key.indices().at(self.value.get_mut(), |_| true) }
    }

    #[inline]
    unsafe fn guard(&self, mask: L) -> Guard<&mut T, L> {
        let inner = Inner(&self.state, mask);
        Guard(unsafe { &mut *self.value.get() }, inner)
    }

    #[inline]
    unsafe fn guard_with<A: At<T>>(&self, mut mask: L, key: &Key<L, A>) -> Guard<A::Item<'_>, L> {
        let mut inner = Inner(&self.state, L::ZERO);
        let item = key.indices().at(self.value.get(), |index| {
            mask.remove(index) && inner.1.add(index)
        });
        // Unlock bits that were not used in the key.
        mask.unlock(&self.state, true);
        Guard(item, inner)
    }
}
