// SPDX-License-Identifier: AGPL-3.0-or-later

//! The interrupt key (Ctrl+C) for the server's children on Windows.
//!
//! A process that a parent starts with `CREATE_NEW_PROCESS_GROUP` ignores
//! Ctrl+C, and its children inherit that. A stop sends Ctrl+C to an agent
//! first, so the server turns the key back on for itself once at start, before
//! it starts any child. On other systems there is nothing to do.

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
