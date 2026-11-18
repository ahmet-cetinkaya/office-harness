// SPDX-License-Identifier: AGPL-3.0-or-later

//! The Windows console calls of the office: the interrupt key (Ctrl+C) for
//! the server's children, and the raw input mode that lets the terminal test
//! fixture read single keys. On other systems both are no-ops.
//!
//! A process that a parent starts with `CREATE_NEW_PROCESS_GROUP` ignores
//! Ctrl+C, and its children inherit that. A stop sends Ctrl+C to an agent
//! first, so the server turns the key back on for itself once at start, before
//! it starts any child.

#[cfg(windows)]
mod windows_console;

/// Makes the children that the server starts from now on react to Ctrl+C.
///
/// # Errors
/// The text of the OS error when Windows refuses the change.
pub fn enable_interrupt_for_children() -> Result<(), String> {
    #[cfg(windows)]
    {
        windows_console::process_ctrl_c()
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Clears the console input modes that hold keys until a whole line, so a
/// program that reads single keys gets each one as it is typed.
///
/// # Errors
/// The text of the OS error when Windows refuses the change, or a note when
/// the process has no input console.
pub fn disable_line_input() -> Result<(), String> {
    #[cfg(windows)]
    {
        windows_console::disable_line_input()
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}
