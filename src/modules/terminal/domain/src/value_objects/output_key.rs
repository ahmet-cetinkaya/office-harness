// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use crate::errors::TerminalError;

const MAX_LENGTH: usize = 64;

/// The name of one session's output file. It becomes a file name, so it holds
/// only letters, digits, `-` and `_`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputKey(String);

impl OutputKey {
    /// # Errors
    /// Returns `InvalidOutputKey` when the text is empty, too long, or holds
    /// another character.
    pub fn parse(text: &str) -> Result<Self, TerminalError> {
        let is_valid = !text.is_empty()
            && text.len() <= MAX_LENGTH
            && text
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character));
        if is_valid {
            return Ok(Self(text.to_owned()));
        }
        Err(TerminalError::InvalidOutputKey {
            max: MAX_LENGTH,
            text: text.chars().take(MAX_LENGTH).collect(),
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OutputKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
#[path = "output_key.tests.rs"]
mod tests;
