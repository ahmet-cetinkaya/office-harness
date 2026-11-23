// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use zeroize::Zeroizing;

const KEY_BYTES: usize = 32;

/// The 256-bit key that seals every secret. Cleared when dropped; prints as
/// `[secret]`; has no `Clone`.
pub struct StoreKey(Zeroizing<[u8; KEY_BYTES]>);

impl StoreKey {
    pub const LENGTH: usize = KEY_BYTES;

    #[must_use]
    pub fn from_bytes(bytes: [u8; KEY_BYTES]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// `None` when the slice is not exactly 32 bytes.
    #[must_use]
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        <[u8; KEY_BYTES]>::try_from(bytes)
            .ok()
            .map(Self::from_bytes)
    }

    #[must_use]
    pub fn expose(&self) -> &[u8; KEY_BYTES] {
        &self.0
    }

    /// Compares without stopping at the first different byte.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0
            .iter()
            .zip(other.0.iter())
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            == 0
    }
}

impl fmt::Debug for StoreKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[secret]")
    }
}

#[cfg(test)]
#[path = "store_key.tests.rs"]
mod tests;
