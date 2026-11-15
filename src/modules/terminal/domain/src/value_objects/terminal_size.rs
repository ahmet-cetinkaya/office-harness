// SPDX-License-Identifier: AGPL-3.0-or-later

use std::num::NonZeroU16;

use crate::errors::TerminalError;

/// The size of a terminal in character cells. Both sides are at least 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    columns: NonZeroU16,
    rows: NonZeroU16,
}

impl TerminalSize {
    /// # Errors
    /// Returns `NotPositive` naming `cols` or `rows` when that side is zero.
    pub fn new(columns: u16, rows: u16) -> Result<Self, TerminalError> {
        let columns =
            NonZeroU16::new(columns).ok_or(TerminalError::NotPositive { field: "cols" })?;
        let rows = NonZeroU16::new(rows).ok_or(TerminalError::NotPositive { field: "rows" })?;
        Ok(Self { columns, rows })
    }

    #[must_use]
    pub const fn columns(self) -> NonZeroU16 {
        self.columns
    }

    #[must_use]
    pub const fn rows(self) -> NonZeroU16 {
        self.rows
    }
}

#[cfg(test)]
#[path = "terminal_size.tests.rs"]
mod tests;
