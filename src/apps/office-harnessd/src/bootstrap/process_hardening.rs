// SPDX-License-Identifier: AGPL-3.0-or-later

//! The server holds secret values in memory (0026/FR-013), so it asks the OS
//! not to write that memory to a file.

/// Sets the size of a core dump of this process to 0 and, on Linux, marks the
/// process not dumpable, which also stops a debugger of the same user and a
/// read of its memory through `/proc`. It is a request to the OS and holds for
/// this process only: a child that the server starts is a new program and
/// takes the defaults of that program.
///
/// # Errors
/// The text of the OS error. Windows has no such switch for a process, so
/// there this does nothing and returns `Ok`.
#[cfg(unix)]
pub fn refuse_memory_dumps() -> Result<(), String> {
    use nix::sys::resource::{Resource, setrlimit};

    setrlimit(Resource::RLIMIT_CORE, 0, 0).map_err(|error| error.to_string())?;
    #[cfg(target_os = "linux")]
    nix::sys::prctl::set_dumpable(false).map_err(|error| error.to_string())?;
    Ok(())
}

/// See the Unix version: Windows has no per-process switch to refuse a dump.
///
/// # Errors
/// None.
#[cfg(not(unix))]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the Unix version can fail, and both have one signature"
)]
pub fn refuse_memory_dumps() -> Result<(), String> {
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "process_hardening.tests.rs"]
mod tests;
