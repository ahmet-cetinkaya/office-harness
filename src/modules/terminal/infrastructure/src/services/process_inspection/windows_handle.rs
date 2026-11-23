// SPDX-License-Identifier: AGPL-3.0-or-later

//! A process handle on Windows. One handle, opened once, reads the creation
//! time and the command line and ends the process, so the values that were
//! checked belong to the process that is ended. A process id that Windows
//! hands out again cannot reach a process through this handle.
#![allow(
    unsafe_code,
    reason = "process handle reads and TerminateProcess have no safe wrapper in windows-sys"
)]

use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::time::Duration;

use terminal_application::common::interfaces::{EndOutcome, ProcessHandle, ReadError};
use terminal_application::common::models::ProcessIdentity;
use windows_sys::Wdk::System::Threading::{
    NtQueryInformationProcess, ProcessCommandLineInformation,
};
use windows_sys::Win32::Foundation::{
    ERROR_INVALID_PARAMETER, FILETIME, STATUS_BUFFER_OVERFLOW, STATUS_BUFFER_TOO_SMALL,
    STATUS_INFO_LENGTH_MISMATCH, UNICODE_STRING, WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};

/// Reading, ending, and waiting on one process (`SYNCHRONIZE` is the right
/// that `WaitForSingleObject` needs).
const ACCESS: u32 = PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE;

/// The exit code of a process that the office ends by force.
const FORCED_EXIT_CODE: u32 = 1;

/// First guess for the command line buffer, in 8-byte words (2 KiB). The
/// call tells the size that it needs when this is too small.
const INITIAL_BUFFER_WORDS: usize = 256;
const BYTES_PER_WORD: usize = size_of::<u64>();

/// The longest finite wait; `u32::MAX` means "wait forever" to Windows.
const LONGEST_WAIT_MILLIS: u32 = u32::MAX - 1;

const EMPTY_FILETIME: FILETIME = FILETIME {
    dwLowDateTime: 0,
    dwHighDateTime: 0,
};

pub struct WindowsHandle {
    process: OwnedHandle,
    pid: u32,
}

impl WindowsHandle {
    /// `NotRunning` only when Windows says that no process has this id;
    /// every other failure (access denied among them) is `NotReadable`.
    pub fn open(pid: u32) -> Result<Box<dyn ProcessHandle>, ReadError> {
        // SAFETY: the call takes integers and returns a new handle or null.
        let raw = unsafe { OpenProcess(ACCESS, 0, pid) };
        if raw.is_null() {
            return Err(open_failure(&io::Error::last_os_error()));
        }
        // SAFETY: `raw` is a handle that nothing else owns; `OwnedHandle`
        // closes it once.
        let process = unsafe { OwnedHandle::from_raw_handle(raw) };
        Ok(Box::new(Self { process, pid }))
    }

    fn has_exited(&self) -> bool {
        self.wait_for_exit(Duration::ZERO)
    }

    /// The creation time in 100 ns units, at full precision.
    fn creation_time(&self) -> Option<u64> {
        let (mut created, mut exited, mut kernel, mut user) = (
            EMPTY_FILETIME,
            EMPTY_FILETIME,
            EMPTY_FILETIME,
            EMPTY_FILETIME,
        );
        // SAFETY: the handle is open for as long as `self` lives, and the four
        // pointers name `FILETIME` values on this stack frame.
        let succeeded = unsafe {
            GetProcessTimes(
                self.process.as_raw_handle(),
                &raw mut created,
                &raw mut exited,
                &raw mut kernel,
                &raw mut user,
            )
        };
        (succeeded != 0)
            .then(|| (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }

    /// The raw command line text, read on the handle.
    fn command_line(&self) -> Option<String> {
        let mut buffer = vec![0_u64; INITIAL_BUFFER_WORDS];
        // One retry with the size that the first call asked for.
        for _ in 0..2 {
            let capacity = buffer.len() * BYTES_PER_WORD;
            let mut needed = 0_u32;
            // SAFETY: the handle is open; `buffer` is writable for `capacity`
            // bytes and 8-byte aligned; `needed` is a `u32` on this stack frame.
            let status = unsafe {
                NtQueryInformationProcess(
                    self.process.as_raw_handle(),
                    ProcessCommandLineInformation,
                    buffer.as_mut_ptr().cast(),
                    u32::try_from(capacity).ok()?,
                    &raw mut needed,
                )
            };
            if status >= 0 {
                return text_in(&buffer);
            }
            let too_small = [
                STATUS_INFO_LENGTH_MISMATCH,
                STATUS_BUFFER_OVERFLOW,
                STATUS_BUFFER_TOO_SMALL,
            ]
            .contains(&status);
            let needed_bytes = usize::try_from(needed).ok()?;
            if !too_small || needed_bytes <= capacity {
                return None;
            }
            buffer = vec![0_u64; needed_bytes.div_ceil(BYTES_PER_WORD)];
        }
        None
    }
}

/// `OpenProcess` answers `ERROR_INVALID_PARAMETER` for an id that no process
/// has; anything else, access denied included, leaves a process that may run.
fn open_failure(error: &io::Error) -> ReadError {
    if error.raw_os_error() == i32::try_from(ERROR_INVALID_PARAMETER).ok() {
        ReadError::NotRunning
    } else {
        ReadError::NotReadable
    }
}

/// Decodes the `UNICODE_STRING` that the kernel wrote at the start of
/// `buffer`, after checking that its text lies inside `buffer`.
fn text_in(buffer: &[u64]) -> Option<String> {
    let base = buffer.as_ptr() as usize;
    let end = base + buffer.len() * BYTES_PER_WORD;
    // SAFETY: the kernel wrote a `UNICODE_STRING` at the start of the buffer,
    // which is 8-byte aligned and larger than the structure.
    let header = unsafe { &*buffer.as_ptr().cast::<UNICODE_STRING>() };
    let start = header.Buffer as usize;
    let byte_length = usize::from(header.Length);
    if byte_length == 0 {
        return Some(String::new());
    }
    if start < base
        || start.checked_add(byte_length)? > end
        || !start.is_multiple_of(align_of::<u16>())
    {
        return None;
    }
    // SAFETY: the range was checked to lie inside `buffer` and to be aligned
    // for `u16`; the kernel wrote that many bytes of text there.
    let units = unsafe { std::slice::from_raw_parts(header.Buffer, byte_length / 2) };
    Some(String::from_utf16_lossy(units))
}

impl ProcessHandle for WindowsHandle {
    fn identity(&self) -> Result<ProcessIdentity, ReadError> {
        if self.has_exited() {
            return Err(ReadError::NotRunning);
        }
        let started_at = self.creation_time().ok_or(ReadError::NotReadable)?;
        let command_line = self
            .command_line()
            .filter(|line| !line.is_empty())
            .ok_or(ReadError::NotReadable)?;
        let boot = super::machine_run().ok_or(ReadError::NotReadable)?;
        Ok(ProcessIdentity::new(
            self.pid,
            started_at,
            &command_line,
            boot,
        ))
    }

    /// A detached process has no console, so no polite request exists.
    fn request_end(&self) -> Result<EndOutcome, String> {
        if self.has_exited() {
            return Ok(EndOutcome::Gone);
        }
        Ok(EndOutcome::Unsupported)
    }

    fn force_end(&self) -> Result<EndOutcome, String> {
        if self.has_exited() {
            return Ok(EndOutcome::Gone);
        }
        // SAFETY: the handle is open and was opened with `PROCESS_TERMINATE`.
        let succeeded = unsafe { TerminateProcess(self.process.as_raw_handle(), FORCED_EXIT_CODE) };
        if succeeded != 0 {
            return Ok(EndOutcome::Delivered);
        }
        let error = io::Error::last_os_error();
        // The process may have exited between the check and the call.
        if self.has_exited() {
            return Ok(EndOutcome::Gone);
        }
        Err(error.to_string())
    }

    fn wait_for_exit(&self, timeout: Duration) -> bool {
        let millis = u32::try_from(timeout.as_millis()).unwrap_or(LONGEST_WAIT_MILLIS);
        // SAFETY: the handle is open and has the `SYNCHRONIZE` right.
        unsafe { WaitForSingleObject(self.process.as_raw_handle(), millis) == WAIT_OBJECT_0 }
    }
}
