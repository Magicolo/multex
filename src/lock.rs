use crate::system;
use std::{
    alloc::{alloc, dealloc, Layout},
    mem::size_of,
    ops::Deref,
    ptr::{drop_in_place, null_mut},
    slice::from_raw_parts,
    sync::{
        atomic::{AtomicPtr, AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering::*},
        Arc,
    },
};

pub trait Mask: Sized {
    fn new() -> Self;
    fn add(&mut self, index: usize) -> bool;
    fn remove(&mut self, index: usize) -> bool;
    fn clear(&mut self);
}

pub unsafe trait Lock: Mask {
    type State;
    const NEW: Self::State;

    fn lock(&self, state: &Self::State, taken: &mut Self, partial: bool, wait: bool) -> bool;
    fn unlock(&self, state: &Self::State, wake: bool) -> bool;
    fn is_locked(&self, state: &Self::State, partial: bool) -> bool;
}

pub trait LockAll: Lock {
    const ALL: Self;
}

#[repr(transparent)]
pub struct Same<T: ?Sized>(pub T);
pub struct State<T>(AtomicPtr<Header<T>>, AtomicU32);

struct Header<T>(usize, *mut Self);

const WAIT: u32 = 1 << 0;

impl<T> Deref for Same<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

macro_rules! lock {
    ($v:ty, $a:ty) => {
        unsafe impl Lock for $v {
            type State = $a;
            #[allow(clippy::declare_interior_mutable_const)]
            const NEW: Self::State = <$a>::new(0);

            fn lock(
                &self,
                state: &Self::State,
                taken: &mut Self,
                partial: bool,
                wait: bool,
            ) -> bool {
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
                //         } else if old & CONTENTION == CONTENTION {
                //             system::wait(state, old, mask);
                //             old = state.load(Acquire);
                //         } else {
                //             old = spin(state, mask, state.fetch_or(CONTENTION, Acquire));
                //             continue;
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
                    *taken = 0;
                    true
                } else if partial {
                    *taken = state.fetch_or(mask, Acquire) ^ mask & mask;
                    true
                } else if wait {
                    *taken = lock_wait(state, mask);
                    true
                } else if lock_once(state, mask).is_ok() {
                    *taken = mask;
                    true
                } else {
                    false
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
            //     if mask == 0 {
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
                if mask == 0 {
                    false
                } else if partial {
                    state.load(Relaxed) & mask != 0
                } else {
                    state.load(Relaxed) & mask == mask
                }
            }
        }

        impl LockAll for $v {
            const ALL: Self = !0;
        }

        impl Mask for $v {
            #[inline]
            fn new() -> Self {
                0
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

            #[inline]
            fn clear(&mut self) {
                *self = 0;
            }
        }

        impl Same<$a> {
            #[allow(clippy::declare_interior_mutable_const)]
            const NEW: Self = Self(<$v>::NEW);
        }

        unsafe impl<const N: usize> Lock for [$v; N] {
            type State = ([Same<$a>; N], AtomicU32);
            #[allow(clippy::declare_interior_mutable_const)]
            const NEW: Self::State = ([Same::<$a>::NEW; N], AtomicU32::new(0));

            #[inline]
            fn lock(
                &self,
                (states, version): &Self::State,
                taken: &mut Self,
                partial: bool,
                wait: bool,
            ) -> bool {
                lock_all(states, version, self, taken, partial, wait)
            }

            #[inline]
            fn unlock(&self, (states, version): &Self::State, wake: bool) -> bool {
                unlock_all(states, version, self, wake)
            }

            #[inline]
            fn is_locked(&self, (states, _): &Self::State, partial: bool) -> bool {
                are_locked(states, self, partial)
            }
        }

        impl<const N: usize> LockAll for [$v; N] {
            const ALL: Self = [0; N];
        }

        impl<const N: usize> Mask for [$v; N] {
            #[inline]
            fn new() -> Self {
                [0; N]
            }

            #[inline]
            fn add(&mut self, index: usize) -> bool {
                match self.get_mut(index / <$v>::BITS as usize) {
                    Some(mask) => mask.add(index % <$v>::BITS as usize),
                    None => false,
                }
            }

            #[inline]
            fn remove(&mut self, index: usize) -> bool {
                match self.get_mut(index / <$v>::BITS as usize) {
                    Some(mask) => mask.remove(index % <$v>::BITS as usize),
                    None => false,
                }
            }

            #[inline]
            fn clear(&mut self) {
                *self = [0; N];
            }
        }

        // TODO: The implementation could be for `[T]`?
        unsafe impl Lock for Vec<$v> {
            type State = State<$a>;
            #[allow(clippy::declare_interior_mutable_const)]
            const NEW: Self::State = State(AtomicPtr::new(null_mut()), AtomicU32::new(0));

            #[inline]
            fn lock(
                &self,
                state: &Self::State,
                taken: &mut Self,
                partial: bool,
                wait: bool,
            ) -> bool {
                taken.resize(self.len(), 0);
                let states = load(&state.0, self.len());
                lock_all(states, &state.1, self, taken, partial, wait)
            }

            #[inline]
            fn unlock(&self, state: &Self::State, wake: bool) -> bool {
                let states = load(&state.0, self.len());
                unlock_all(states, &state.1, self, wake)
            }

            #[inline]
            fn is_locked(&self, State(state, _): &Self::State, partial: bool) -> bool {
                let states = load(state, self.len());
                are_locked(states, self, partial)
            }
        }

        impl Mask for Vec<$v> {
            #[inline]
            fn new() -> Self {
                Vec::new()
            }

            #[inline]
            fn add(&mut self, index: usize) -> bool {
                loop {
                    match self.get_mut(index / usize::BITS as usize) {
                        Some(mask) => break mask.add(index % usize::BITS as usize),
                        None => self.push(0),
                    }
                }
            }

            #[inline]
            fn remove(&mut self, index: usize) -> bool {
                match self.get_mut(index / usize::BITS as usize) {
                    Some(mask) => mask.remove(index % usize::BITS as usize),
                    None => false,
                }
            }

            #[inline]
            fn clear(&mut self) {
                self.clear()
            }
        }
    };
}

lock!(u8, AtomicU8);
lock!(u16, AtomicU16);
lock!(u32, AtomicU32);
lock!(u64, AtomicU64);
lock!(usize, AtomicUsize);

impl<T> Drop for State<T> {
    fn drop(&mut self) {
        free_all(*self.0.get_mut());
    }
}

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

fn lock_all<L: Lock, D: Deref<Target = L::State>>(
    states: &[D],
    version: &AtomicU32,
    mask: &[L],
    locks: &mut [L],
    partial: bool,
    wait: bool,
) -> bool {
    'outer: loop {
        let value = if wait {
            version.load(Acquire)
        } else {
            u32::MAX
        };
        for (index, pair) in states.iter().zip(mask).enumerate() {
            let (head, tail) = locks.split_at_mut(index);
            let Some((lock, _)) = tail.split_first_mut() else {
                unreachable!()
            };
            if pair.1.lock(pair.0, lock, partial, false) {
                continue;
            } else if wait && value & WAIT == WAIT {
                unlock_all(states, version, head, true);
                system::wait(version, value, bit(index));
                continue 'outer;
            } else if wait {
                unlock_all(states, version, head, true);
                version.fetch_or(WAIT, Release);
                continue 'outer;
            } else {
                unlock_all(states, version, head, true);
                break 'outer false;
            }
        }
        break true;
    }
}

fn unlock_all<L: Lock, D: Deref<Target = L::State>>(
    states: &[D],
    version: &AtomicU32,
    masks: &[L],
    wake: bool,
) -> bool {
    let mut mask = 0;
    for (index, pair) in states.iter().zip(masks).enumerate() {
        if pair.1.unlock(pair.0, false) {
            mask |= bit(index);
        }
    }
    self::wake(version, mask, wake)
}

fn are_locked<L: Lock, D: Deref<Target = L::State>>(
    states: &[D],
    masks: &[L],
    partial: bool,
) -> bool {
    for (state, mask) in states.iter().zip(masks) {
        if mask.is_locked(state, partial) {
            return true;
        }
    }
    false
}

fn grow<T: Default>(
    state: &AtomicPtr<Header<T>>,
    old: *mut Header<T>,
    counts: (usize, usize),
) -> *mut Header<T> {
    debug_assert!(counts.0 >= counts.1);
    let (layout, offset) = Layout::new::<Header<T>>()
        .extend(Layout::array::<Arc<T>>(counts.1).unwrap())
        .unwrap();
    debug_assert_eq!(offset, size_of::<Header<T>>());

    let data = unsafe { alloc(layout).cast::<Header<T>>() };
    unsafe { data.write(Header(counts.1, old)) };

    for index in 0..counts.0 {
        let source = unsafe { old.add(1).cast::<Arc<T>>().add(index) };
        let target = unsafe { data.add(1).cast::<Arc<T>>().add(index) };
        unsafe { target.write(Arc::clone(&*source)) };
    }

    for index in counts.0..counts.1 {
        let target = unsafe { data.add(1).cast::<Arc<T>>().add(index) };
        unsafe { target.write(Arc::new(T::default())) };
    }

    match state.compare_exchange_weak(old, data, Acquire, Relaxed) {
        Ok(new) => new,
        Err(new) => {
            free_one(data, counts.1, layout);
            new
        }
    }
}

fn load<T: Default>(state: &AtomicPtr<Header<T>>, count: usize) -> &[Arc<T>] {
    let mut old = state.load(Acquire);
    loop {
        if old.is_null() {
            old = grow(state, old, (0, count));
            continue;
        }

        let capacity = unsafe { &*old }.0;
        if capacity < count {
            old = grow(state, old, (capacity, count));
            continue;
        }

        break unsafe { from_raw_parts(old.add(1).cast::<Arc<T>>(), capacity) };
    }
}

fn free_all<T>(mut data: *mut Header<T>) {
    // TODO: How can the old pointers be freed before the `Multex` is freed?
    while let Some(&Header(count, next)) = unsafe { data.as_ref() } {
        debug_assert!(count > 0);
        let (layout, offset) = Layout::new::<Header<T>>()
            .extend(Layout::array::<Arc<T>>(count).unwrap())
            .unwrap();
        debug_assert_eq!(offset, size_of::<Header<T>>());
        free_one(data, count, layout);
        data = next;
    }
}

fn free_one<T>(data: *mut Header<T>, count: usize, layout: Layout) {
    for index in 0..count {
        unsafe { drop_in_place(data.add(1).cast::<Arc<T>>().add(index)) };
    }
    unsafe { dealloc(data.cast(), layout) };
}
