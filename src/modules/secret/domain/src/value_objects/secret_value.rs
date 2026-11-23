// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use zeroize::Zeroizing;

use crate::errors::SecretError;

/// The shortest value (FR-016): a shorter one would replace unrelated text in
/// stored output (spec A4).
pub const MIN_VALUE_BYTES: usize = 8;
pub const MAX_VALUE_BYTES: usize = 8192;

/// The text of a secret. It is cleared when dropped, has no `Clone`,
/// `Display`, or `Serialize`, and prints as `[secret]`. `reveal` is the one
/// way out and the caller names the reason by calling it.
pub struct SecretValue(Zeroizing<String>);

impl SecretValue {
    /// # Errors
    /// `ValueTooShort`, `ValueTooLong`, or `ValueHasNul`; no error holds the value.
    pub fn parse(text: String) -> Result<Self, SecretError> {
        let text = Zeroizing::new(text);
        if text.len() < MIN_VALUE_BYTES {
            return Err(SecretError::ValueTooShort {
                min: MIN_VALUE_BYTES,
            });
        }
        if text.len() > MAX_VALUE_BYTES {
            return Err(SecretError::ValueTooLong {
                max: MAX_VALUE_BYTES,
            });
        }
        if text.contains('\0') {
            return Err(SecretError::ValueHasNul);
        }
        Ok(Self(text))
    }

    /// The value as text. Only a granted read and the sealing call this.
    #[must_use]
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[secret]")
    }
}

#[cfg(test)]
#[path = "secret_value.tests.rs"]
mod tests;
