// SPDX-License-Identifier: AGPL-3.0-or-later

use super::office_token::RANDOM_BYTE_COUNT;

/// The challenge of one connection: 32 random bytes as lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nonce(String);

impl Nonce {
    #[must_use]
    pub fn from_random_bytes(bytes: [u8; RANDOM_BYTE_COUNT]) -> Self {
        Self(hex::encode(bytes))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
