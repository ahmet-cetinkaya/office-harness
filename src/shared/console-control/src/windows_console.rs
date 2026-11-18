// SPDX-License-Identifier: AGPL-3.0-or-later

//! The Windows API calls of this crate. The only `unsafe` code of the
//! server outside the private files.
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode, GetStdHandle,
    STD_INPUT_HANDLE, SetConsoleCtrlHandler, SetConsoleMode,
};

/// `FALSE` with no handler: "process Ctrl+C normally" again.
const RESTORE_NORMAL_PROCESSING: i32 = 0;

pub(crate) fn process_ctrl_c() -> Result<(), String> {
    // SAFETY: `SetConsoleCtrlHandler(NULL, FALSE)` passes no pointer and no
    // callback. With a NULL handler and FALSE, Windows clears the process's
    // "ignore Ctrl+C" attribute, which new child processes inherit. The call
    // touches only that attribute of the calling process.
    let succeeded = unsafe { SetConsoleCtrlHandler(None, RESTORE_NORMAL_PROCESSING) };
    if succeeded == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

/// The input modes that collect characters into lines and echo them; a
/// program that reads single keys clears them, the way `stty raw -echo` does
/// on Unix.
const LINE_INPUT_MODES: u32 = ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT;

pub(crate) fn disable_line_input() -> Result<(), String> {
    let console = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if console.is_null() {
        return Err("the process has no standard input handle".to_owned());
    }
    // A pseudoconsole child gets `INVALID_HANDLE_VALUE` from `GetStdHandle`
    // when its stdin is not a console; there is nothing to change then.
    if console == INVALID_HANDLE_VALUE {
        return Err("the process has no input console".to_owned());
    }
    let mut mode = 0_u32;
    // SAFETY: `console` is a console handle from `GetStdHandle`, and the
    // pointer is the valid `mode` out parameter. The call reads and writes
    // the input mode of the calling process's own console.
    if unsafe { GetConsoleMode(console, std::ptr::addr_of_mut!(mode)) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: `console` is the console handle this process read the mode of;
    // the call changes only that console's input mode.
    if unsafe { SetConsoleMode(console, mode & !LINE_INPUT_MODES) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}
