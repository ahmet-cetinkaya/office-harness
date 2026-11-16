// SPDX-License-Identifier: AGPL-3.0-or-later

/// A question that a program asks its terminal. The server answers these
/// itself, so a CLI gets an answer whether or not a client is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalQuery {
    /// `CSI 6 n`: where is the cursor?
    CursorPosition,
    /// `CSI c`: which terminal are you?
    DeviceAttributes,
    /// `CSI > c`: which terminal version are you?
    SecondaryDeviceAttributes,
    /// `CSI 5 n`: are you working?
    DeviceStatus,
}

impl TerminalQuery {
    /// The bytes to write back to the program. The cursor is zero-based.
    #[must_use]
    pub fn reply(self, cursor_row: u16, cursor_column: u16) -> Vec<u8> {
        match self {
            Self::CursorPosition => format!(
                "\x1b[{};{}R",
                u32::from(cursor_row) + 1,
                u32::from(cursor_column) + 1
            )
            .into_bytes(),
            Self::DeviceAttributes => b"\x1b[?62;22c".to_vec(),
            Self::SecondaryDeviceAttributes => b"\x1b[>0;0;0c".to_vec(),
            Self::DeviceStatus => b"\x1b[0n".to_vec(),
        }
    }
}

#[cfg(test)]
#[path = "terminal_query.tests.rs"]
mod tests;
