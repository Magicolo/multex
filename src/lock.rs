use crate::system;
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering::*},
};

pub unsafe trait Lock: Sized {
    type State;
    const MAX: Self;
    const ZERO: Self;
    const BITS: usize;
    const NEW: Self::State;

    fn lock(&self, state: &Self::State, partial: bool, wait: bool) -> Option<Self>;
    fn unlock(&self, state: &Self::State, wake: bool) -> bool;
    fn is_locked(&self, state: &Self::State, partial: bool) -> bool;
    fn add(&mut self, index: usize) -> bool;
    fn remove(&mut self, index: usize) -> bool;
}

struct Unlock<T = ()>(PhantomData<T>);
const WAIT: u32 = 1 << 0;

impl<T> Unlock<T> {
    #[inline]
    const fn bit(index: usize) -> u32 {
        1 << (index % u32::BITS as usize)
    }

    #[inline]
    fn wake(state: &AtomicU32, mask: u32, wake: bool) -> bool {
        if mask == 0 {
            false
        } else if wake {
            let value = state.fetch_and(!WAIT, Release);
            if value & WAIT == WAIT {
                state.fetch_add(2, Release);
                system::wake(state, mask);
            }
            true
        } else {
            true
        }
    }
}

macro_rules! lock {
    ($v:ty, $a:ty) => {
        unsafe impl Lock for $v {
            type State = $a;
            const MAX: Self = Self::MAX;
            const ZERO: Self = 0;
            const BITS: usize = Self::BITS as usize;
            #[allow(clippy::declare_interior_mutable_const)]
            const NEW: Self::State = <$a>::new(0);

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

                #[inline]
                fn lock_wait(state: &$a, mask: $v) -> $v {
                    loop {
                        match lock_once(state, mask) {
                            Ok(_) => break mask,
                            Err(value) => system::wait(state, value, mask),
                        }
                    }
                }

                // const CONTENTION: $v = (1 as $v << (<$v>::BITS as usize - 1));
                // fn lock_wait(state: &$a, mask: $v) -> $v {
                //     let mut old = state.load(Acquire);
                //     loop {
                //         if old & mask == 0 {
                //             match state.compare_exchange_weak(old, old | mask, Acquire, Relaxed) {
                //                 Ok(_) => break mask,
                //                 Err(value) => { old = value; continue; }
                //             }
                //         } else if old & CONTENTION == 0 {
                //             old = spin(state, mask, state.fetch_or(CONTENTION, Acquire));
                //             continue;
                //         } else {
                //             system::wait(state, old, mask);
                //             old = state.load(Acquire);
                //         }
                //     }
                // }

                // fn spin(state: &$a, mask: $v, mut old: $v) -> $v {
                //     for _ in 0..10 {
                //         let new = old & mask;
                //         if new == 0 {
                //             break;
                //         } else {
                //             hint::spin_loop();
                //             old = state.load(Relaxed);
                //         }
                //     }
                //     old
                // }

                let mask = *self;
                if mask == 0 {
                    Some(mask)
                } else if partial {
                    Some(state.fetch_or(mask, Acquire) ^ mask & mask)
                } else if wait {
                    Some(lock_wait(state, mask))
                } else if lock_once(state, mask).is_ok() {
                    Some(mask)
                } else {
                    None
                }
            }

            #[inline]
            fn unlock(&self, state: &Self::State, wake: bool) -> bool {
                let mask = *self;
                if mask == 0 {
                    return false;
                }

                let value = state.fetch_and(!mask, Release);
                if value & mask == 0 {
                    false
                } else if wake {
                    system::wake(state, mask);
                    true
                } else {
                    true
                }
            }

            // #[inline]
            // fn unlock(&self, state: &Self::State, wake: bool) -> bool {
            //     const CONTENTION: $v = (1 as $v << (<$v>::BITS as usize - 1));

            //     let mask = *self;
            //     if mask == Self::ZERO {
            //         return false;
            //     }
            //     let bits = mask | CONTENTION;
            //     let value = state.fetch_and(!bits, Release);
            //     if wake && value & CONTENTION == CONTENTION {
            //         system::wake(state, mask);
            //     }
            //     value & bits != 0
            // }

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
            fn add(&mut self, index: usize) -> bool {
                let Some(bit) = (1 as $v).checked_shl(index as _) else {
                    return false;
                };
                let previous = *self;
                *self |= bit;
                previous != *self
            }

            #[inline]
            fn remove(&mut self, index: usize) -> bool {
                let Some(bit) = (1 as $v).checked_shl(index as _) else {
                    return false;
                };
                let previous = *self;
                *self &= !bit;
                previous != *self
            }
        }
    };
}

lock!(u8, AtomicU8);
lock!(u16, AtomicU16);
lock!(u32, AtomicU32);
lock!(u64, AtomicU64);
lock!(usize, AtomicUsize);

impl<L: Lock, const N: usize> Unlock<[L; N]> {
    fn unlock(state: &<[L; N] as Lock>::State, masks: &[L], wake: bool) -> bool {
        let mut mask = 0;
        for (index, pair) in state.0.iter().zip(masks).enumerate() {
            if pair.1.unlock(pair.0, false) {
                mask |= Self::bit(index);
            }
        }
        Self::wake(&state.1, mask, wake)
    }
}

unsafe impl<L: Lock, const N: usize> Lock for [L; N] {
    type State = ([L::State; N], AtomicU32);
    const MAX: Self = [L::MAX; N];
    const ZERO: Self = [L::ZERO; N];
    const BITS: usize = L::BITS * N;
    const NEW: Self::State = ([L::NEW; N], AtomicU32::new(0));

    fn lock(&self, state: &Self::State, partial: bool, wait: bool) -> Option<Self> {
        'outer: loop {
            let mut masks = Self::ZERO;
            let value = if wait {
                state.1.load(Acquire)
            } else {
                u32::MAX
            };
            for (index, pair) in state.0.iter().zip(self).enumerate() {
                match pair.1.lock(pair.0, partial, false) {
                    Some(mask) => masks[index] = mask,
                    None if wait && value & WAIT == WAIT => {
                        Unlock::<Self>::unlock(state, &masks[..index], true);
                        system::wait(&state.1, value, Unlock::<Self>::bit(index));
                        continue 'outer;
                    }
                    None if wait => {
                        Unlock::<Self>::unlock(state, &masks[..index], true);
                        state.1.fetch_or(WAIT, Release);
                        continue 'outer;
                    }
                    None => {
                        Unlock::<Self>::unlock(state, &masks[..index], true);
                        break 'outer None;
                    }
                }
            }
            break Some(masks);
        }
    }

    #[inline]
    fn unlock(&self, state: &Self::State, wake: bool) -> bool {
        Unlock::<Self>::unlock(state, self, wake)
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
    fn add(&mut self, index: usize) -> bool {
        match self.get_mut(index / L::BITS) {
            Some(mask) => mask.add(index % L::BITS),
            None => false,
        }
    }

    #[inline]
    fn remove(&mut self, index: usize) -> bool {
        match self.get_mut(index / L::BITS) {
            Some(mask) => mask.remove(index % L::BITS),
            None => false,
        }
    }
}

macro_rules! tuples {
    ($n:tt, $($l:ident, $i:tt),+) => {
        impl<$($l: Lock),+> Unlock<($($l,)+)> {
            pub fn unlock(state: &<($($l,)+) as Lock>::State, masks: &($($l,)+), count: usize, wake: bool) -> bool {
                let mut mask = 0;
                'main: { $(if $i < count { if masks.$i.unlock(&state.$i, false) { mask |= Self::bit($i); } } else { break 'main; })+ }
                Self::wake(&state.$n, mask, wake)
            }
        }

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
                        None if wait && value & WAIT == WAIT => {
                            // TODO: unlock.
                            Unlock::<Self>::unlock(state, &masks, $i, true);
                            system::wait(&state.1, value, Unlock::<Self>::bit($i));
                            continue;
                        }
                        None if wait => {
                            // TODO: unlock.
                            state.$n.fetch_or(WAIT, Release);
                            continue;
                        }
                        None => {
                            // TODO: unlock.
                            break None;
                        }
                    })+
                    break Some(masks);
                }
            }
            #[inline]
            fn unlock(&self, state: &Self::State, wake: bool) -> bool {
                Unlock::<Self>::unlock(state, self, $n, wake)
            }
            #[inline]
            fn is_locked(&self, state: &Self::State, partial: bool) -> bool {
                $(if self.$i.is_locked(&state.$i, partial) { return true; })+
                false
            }
            #[inline]
            fn add(&mut self, mut _index: usize) -> bool {
                $(if _index < $l::BITS { return self.$i.add(_index); } else { _index -= $l::BITS; })+
                false
            }
            #[inline]
            fn remove(&mut self, mut _index: usize) -> bool {
                $(if _index < $l::BITS { return self.$i.remove(_index); } else { _index -= $l::BITS; })+
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
