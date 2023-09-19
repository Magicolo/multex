use crate::system;
use std::{
    error::Error,
    fmt,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering::*},
};

pub unsafe trait Lock: Sized {
    type State;
    const MAX: Self;
    const ZERO: Self;
    const BITS: usize;
    const NEW: Self::State;

    fn lock(&self, state: &Self::State, partial: bool, wait: bool) -> Option<Self>;
    fn unlock(&self, state: &Self::State, wake: bool);
    fn is_locked(&self, state: &Self::State, partial: bool) -> bool;
    fn add(&mut self, index: usize) -> Result<(), IndexError>;
    fn has(&self, index: usize) -> bool;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum IndexError {
    OutOfBounds(usize),
    Duplicate(usize),
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl Error for IndexError {}

macro_rules! lock {
    ($v:ty, $a:ty) => {
        unsafe impl Lock for $v {
            type State = $a;
            const MAX: Self = Self::MAX;
            const ZERO: Self = 0;
            const BITS: usize = Self::BITS as usize;
            #[allow(clippy::declare_interior_mutable_const)]
            const NEW: Self::State = <$a>::new(0);

            #[inline]
            fn lock(&self, state: &Self::State, partial: bool, wait: bool) -> Option<Self> {
                #[inline]
                fn lock_once(state: &$a, mask: $v) -> Result<$v, $v> {
                    state.fetch_update(Acquire, Relaxed, |state| {
                        if state & mask == 0 {
                            Some(state | mask)
                        } else {
                            None
                        }
                    })
                }

                fn lock_wait(state: &$a, mask: $v) -> $v {
                    loop {
                        match lock_once(state, mask) {
                            Ok(_) => break mask,
                            Err(value) => system::wait(state, value, mask),
                        }
                    }
                }

                let mask = *self;
                if mask == Self::ZERO {
                    Some(mask)
                } else if partial {
                    Some(state.fetch_or(mask, Acquire) ^ mask & mask)
                } else if wait {
                    Some(lock_wait(state, mask))
                } else {
                    lock_once(state, mask).ok()
                }
            }

            #[inline]
            fn unlock(&self, state: &Self::State, wake: bool) {
                let mask = *self;
                if mask == Self::ZERO {
                    return;
                }

                state.fetch_and(!mask, Release);
                if wake {
                    system::wake(state, mask);
                }
            }

            #[inline]
            fn is_locked(&self, state: &Self::State, partial: bool) -> bool {
                let mask = *self;
                if mask == Self::ZERO {
                    false
                } else if partial {
                    state.load(Relaxed) & mask != Self::ZERO
                } else {
                    state.load(Relaxed) & mask == mask
                }
            }

            #[inline]
            fn add(&mut self, index: usize) -> Result<(), IndexError> {
                let Some(bit) = (1 as $v).checked_shl(index as _) else { return Err(IndexError::OutOfBounds(index)) };
                let previous = *self;
                *self |= bit;
                if previous == *self {
                    Err(IndexError::Duplicate(index))
                } else {
                    Ok(())
                }
            }

            #[inline]
            fn has(&self, index: usize) -> bool {
                let Some(bit) = (1 as $v).checked_shl(index as _) else { return false; };
                self & bit == bit
            }
        }
    };
}

lock!(u8, AtomicU8);
lock!(u16, AtomicU16);
lock!(u32, AtomicU32);
lock!(u64, AtomicU64);
lock!(usize, AtomicUsize);

unsafe impl<L: Lock, const N: usize> Lock for [L; N] {
    type State = ([L::State; N], AtomicU32);
    const MAX: Self = [L::MAX; N];
    const ZERO: Self = [L::ZERO; N];
    const BITS: usize = L::BITS * N;
    const NEW: Self::State = ([L::NEW; N], AtomicU32::new(0));

    fn lock(&self, state: &Self::State, partial: bool, wait: bool) -> Option<Self> {
        loop {
            let mut masks = Self::ZERO;
            let mut done = true;
            let value = if wait {
                state.1.load(Acquire)
            } else {
                u32::MAX
            };
            for (index, pair) in state.0.iter().zip(self).enumerate() {
                match pair.1.lock(pair.0, partial, false) {
                    Some(mask) => masks[index] = mask,
                    None => {
                        // The `masks` ensures that only locks that were taken are unlocked.
                        masks.unlock(state, true);
                        if wait {
                            system::wait(&state.1, value, u32::MAX);
                            done = false;
                            break;
                        } else {
                            return None;
                        }
                    }
                }
            }
            if done {
                return Some(masks);
            }
        }
    }

    #[inline]
    fn unlock(&self, state: &Self::State, wake: bool) {
        for (state, mask) in state.0.iter().zip(self) {
            mask.unlock(state, false);
        }
        state.1.fetch_add(1, Release);
        if wake {
            system::wake(&state.1, u32::MAX);
        }
    }

    #[inline]
    fn is_locked(&self, state: &Self::State, partial: bool) -> bool {
        for (state, mask) in state.0.iter().zip(self) {
            if mask.is_locked(state, partial) {
                return true;
            }
        }
        false
    }

    #[inline]
    fn add(&mut self, index: usize) -> Result<(), IndexError> {
        match self.get_mut(index / L::BITS) {
            Some(mask) => mask.add(index % L::BITS),
            None => Err(IndexError::OutOfBounds(index)),
        }
    }

    #[inline]
    fn has(&self, index: usize) -> bool {
        match self.get(index / L::BITS) {
            Some(mask) => mask.has(index % L::BITS),
            None => false,
        }
    }
}

macro_rules! tuples {
    ($n:tt, $($l:ident, $i:tt),+) => {
        unsafe impl<$($l: Lock),+> Lock for ($($l,)+) {
            type State = ($($l::State,)+ AtomicU32);
            const MAX: Self = ($($l::MAX,)+);
            const ZERO: Self = ($($l::ZERO,)+);
            const BITS: usize = $($l::BITS +)+ 0;
            const NEW: Self::State = ($($l::NEW,)+ AtomicU32::new(0));

            fn lock(&self, state: &Self::State, partial: bool, wait: bool) -> Option<Self> {
                loop {
                    let mut masks = Self::ZERO;
                    let value = if wait { state.$n.load(Acquire) } else { u32::MAX };
                    $(match self.$i.lock(&state.$i, partial, false) {
                        Some(mask) => masks.$i = mask,
                        None => {
                            // The `masks` ensures that only locks that were taken are unlocked.
                            masks.unlock(state, true);
                            if wait { system::wait(&state.1, value, u32::MAX); continue; }
                            else { break None; }
                        }
                    })+
                    break Some(masks);
                }
            }
            #[inline]
            fn unlock(&self, state: &Self::State, wake: bool) {
                $(self.$i.unlock(&state.$i, false);)+
                state.$n.fetch_and(1, Release);
                if wake {
                    system::wake(&state.$n, u32::MAX);
                }
            }
            #[inline]
            fn is_locked(&self, state: &Self::State, partial: bool) -> bool {
                $(if self.$i.is_locked(&state.$i, partial) { return true; })+
                false
            }
            #[inline]
            fn add(&mut self, mut index: usize) -> Result<(), IndexError> {
                $(if index < $l::BITS { self.$i.add(index)?; return Ok(()); } else { index -= $l::BITS; })+
                Err(IndexError::OutOfBounds(index))
            }
            #[inline]
            fn has(&self, mut _index: usize) -> bool {
                $(if _index < $l::BITS { return self.$i.has(_index); } else { _index -= $l::BITS; })+
                false
            }
        }
    };
}

tuples!(1, L1, 0);
tuples!(2, L1, 0, L2, 1);
tuples!(3, L1, 0, L2, 1, L3, 2);
tuples!(4, L1, 0, L2, 1, L3, 2, L4, 3);
tuples!(5, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4);
tuples!(6, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5);
tuples!(7, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6);
tuples!(8, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7);
tuples!(9, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7, L9, 8);
tuples!(10, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7, L9, 8, L10, 9);
tuples!(11, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7, L9, 8, L10, 9, L11, 10);
tuples!(
    12, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7, L9, 8, L10, 9, L11, 10, L12, 11
);
tuples!(
    13, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7, L9, 8, L10, 9, L11, 10, L12, 11,
    L13, 12
);
tuples!(
    14, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7, L9, 8, L10, 9, L11, 10, L12, 11,
    L13, 12, L14, 13
);
tuples!(
    15, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7, L9, 8, L10, 9, L11, 10, L12, 11,
    L13, 12, L14, 13, L15, 14
);
tuples!(
    16, L1, 0, L2, 1, L3, 2, L4, 3, L5, 4, L6, 5, L7, 6, L8, 7, L9, 8, L10, 9, L11, 10, L12, 11,
    L13, 12, L14, 13, L15, 14, L16, 15
);
