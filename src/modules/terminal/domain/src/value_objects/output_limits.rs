// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::TerminalError;

/// The output file never grows past this size: full-screen interfaces redraw
/// without newlines, so the line limit alone would not bound it.
pub const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

/// How much recent output a session keeps: the newest lines, and no more than
/// a byte cap, whichever is reached first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputLimits {
    max_lines: usize,
    max_bytes: usize,
}

impl OutputLimits {
    /// The byte cap is lowered to `MAX_OUTPUT_BYTES` when it is higher.
    ///
    /// # Errors
    /// Returns `NotPositive` when a limit is zero.
    pub fn new(max_lines: usize, max_bytes: usize) -> Result<Self, TerminalError> {
        if max_lines == 0 {
            return Err(TerminalError::NotPositive {
                field: "output_line_limit",
            });
        }
        if max_bytes == 0 {
            return Err(TerminalError::NotPositive {
                field: "output_byte_limit",
            });
        }
        Ok(Self {
            max_lines,
            max_bytes: max_bytes.min(MAX_OUTPUT_BYTES),
        })
    }

    /// The line limit with the largest byte cap.
    ///
    /// # Errors
    /// Returns `NotPositive` when the line limit is zero.
    pub fn with_lines(max_lines: usize) -> Result<Self, TerminalError> {
        Self::new(max_lines, MAX_OUTPUT_BYTES)
    }

    #[must_use]
    pub const fn max_lines(self) -> usize {
        self.max_lines
    }

    #[must_use]
    pub const fn max_bytes(self) -> usize {
        self.max_bytes
    }
}

#[cfg(test)]
#[path = "output_limits.tests.rs"]
mod tests;
