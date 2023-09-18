#[cfg(target_os = "linux")]
#[inline]
pub fn wait<S, V, M>(state: &S, value: V, mask: M) {
    use std::mem::size_of;
    debug_assert_eq!(size_of::<S>(), size_of::<V>());
    debug_assert_eq!(size_of::<S>(), size_of::<M>());
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            state as *const _,
            libc::FUTEX_WAIT_BITSET | libc::FUTEX_PRIVATE_FLAG,
            value,
            null::<libc::timespec>(),
            null::<u32>(),
            mask, // TODO: Convert mask to u32. Deal with overflows. At least 1 bit must be one, otherwise u32::MAX?
        )
    };
}

#[cfg(target_os = "linux")]
#[inline]
pub fn wake<S, M>(state: &S, mask: M) {
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            state as *const _,
            libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG,
            2,
            null::<libc::timespec>(),
            null::<u32>(),
            mask, // TODO: Convert to u32.
        )
    };
}

#[cfg(target_os = "linux")]
#[inline]
pub fn wait_array(state: &AtomicU32, value: u32) {
    use std::mem::size_of;
    debug_assert_eq!(size_of::<S>(), size_of::<V>());
    debug_assert_eq!(size_of::<S>(), size_of::<M>());
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            state as *const _,
            libc::FUTEX_WAIT_BITSET | libc::FUTEX_PRIVATE_FLAG,
            value,
            null::<libc::timespec>(),
            null::<u32>(),
            u32::MAX,
        )
    };
}

#[cfg(target_os = "linux")]
#[inline]
pub fn wake_array(state: &AtomicU32) {
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            state as *const _,
            libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG,
            u32::MAX,
            null::<libc::timespec>(),
            null::<u32>(),
            u32::MAX,
        )
    };
}

#[cfg(target_os = "windows")]
#[inline]
pub fn wait<S, V, M>(state: &S, value: V, _: M) {
    use std::mem::size_of;
    debug_assert_eq!(size_of::<S>(), size_of::<V>());
    unsafe {
        windows_sys::Win32::System::Threading::WaitOnAddress(
            state as *const _ as *const _,
            &value as *const _ as *const _,
            size_of::<S>(),
            u32::MAX,
        )
    };
}

#[cfg(target_os = "windows")]
#[inline]
pub fn wake<S, M>(state: &S, _: M) {
    unsafe {
        windows_sys::Win32::System::Threading::WakeByAddressAll(state as *const _ as *const _)
    };
}
