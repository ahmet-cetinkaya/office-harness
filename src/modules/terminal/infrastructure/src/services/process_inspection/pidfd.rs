// SPDX-License-Identifier: AGPL-3.0-or-later

//! The two pidfd system calls. `nix` 0.28 and `libc` have no wrapper, so they
//! go through `libc::syscall`. Both take integers only, never a pointer to
//! memory that the kernel writes.
#![allow(
    unsafe_code,
    reason = "pidfd_open and pidfd_send_signal have no safe wrapper in nix 0.28"
)]

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

/// The flags argument of both calls: none.
const NO_FLAGS: libc::c_long = 0;

/// Opens a pidfd for the process with this id. `ENOSYS` means a kernel before
/// 5.3; `ESRCH` means that no such process exists.
pub fn open(pid: u32) -> io::Result<OwnedFd> {
    // SAFETY: the call takes two integers and returns a new descriptor or -1.
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::c_long::from(pid), NO_FLAGS) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    let descriptor =
        RawFd::try_from(raw).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
    // SAFETY: the kernel just returned `descriptor`, so it is open and
    // nothing else owns it.
    Ok(unsafe { OwnedFd::from_raw_fd(descriptor) })
}

/// Sends `signal` to the process behind the pidfd, and to no other process.
/// `ESRCH` means that the process has gone.
pub fn send_signal(pidfd: &OwnedFd, signal: libc::c_int) -> io::Result<()> {
    // SAFETY: the descriptor stays open for as long as the borrow lasts; the
    // null `siginfo` pointer makes the kernel build the default information.
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            libc::c_long::from(pidfd.as_raw_fd()),
            libc::c_long::from(signal),
            std::ptr::null::<libc::siginfo_t>(),
            NO_FLAGS,
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
