// SPDX-License-Identifier: AGPL-3.0-or-later

/// The stored form of a sealed value: the nonce and the ciphertext, and
/// nothing readable about the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedRecord(Vec<u8>);

impl SealedRecord {
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
