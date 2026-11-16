// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use crate::errors::TokenError;

/// A token and a nonce are both this many random bytes.
pub const RANDOM_BYTE_COUNT: usize = 32;

/// The length of the hex text of `RANDOM_BYTE_COUNT` bytes.
pub const TOKEN_TEXT_LENGTH: usize = RANDOM_BYTE_COUNT * 2;

/// The secret that proves a client belongs to the office: 32 random bytes as
/// lowercase hex. It has no `Display` and its `Debug` hides the value, so it
/// cannot reach a log or an error message by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct OfficeToken(String);

impl OfficeToken {
    #[must_use]
    pub fn from_random_bytes(bytes: [u8; RANDOM_BYTE_COUNT]) -> Self {
        Self(hex::encode(bytes))
    }

    /// Reads the token file's text; a trailing line break is allowed.
    ///
    /// # Errors
    /// `Malformed` when the text is not 64 lowercase hex characters.
    pub fn parse(text: &str) -> Result<Self, TokenError> {
        let text = text.trim_end_matches(['\n', '\r']);
        let is_lowercase_hex = text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        if text.len() == TOKEN_TEXT_LENGTH && is_lowercase_hex {
            Ok(Self(text.to_owned()))
        } else {
            Err(TokenError::Malformed)
        }
    }

    /// The secret text. Only the file store and the proof code call this.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for OfficeToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OfficeToken(<hidden>)")
    }
}

#[cfg(test)]
#[path = "office_token.tests.rs"]
mod tests;
