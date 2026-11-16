// SPDX-License-Identifier: AGPL-3.0-or-later

//! The one Windows API call of this crate. The only `unsafe` code of the
//! server outside the private files.
#![allow(unsafe_code)]

use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

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
