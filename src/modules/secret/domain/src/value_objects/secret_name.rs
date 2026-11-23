// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use crate::errors::SecretError;

const MAX_NAME_CHARS: usize = 64;
/// The most secrets one office holds (FR-016).
pub const MAX_SECRETS: usize = 100;

/// A secret name: 1 to 64 characters, a letter A to Z first, then A to Z,
/// digits, and underscore (FR-016).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretName(String);

impl SecretName {
    /// # Errors
    /// `NameEmpty`, `NameTooLong`, or `NameBadForm`.
    pub fn parse(text: &str) -> Result<Self, SecretError> {
        if text.is_empty() {
            return Err(SecretError::NameEmpty);
        }
        if text.chars().count() > MAX_NAME_CHARS {
            return Err(SecretError::NameTooLong {
                max: MAX_NAME_CHARS,
            });
        }
        let mut characters = text.chars();
        let first_is_letter = characters.next().is_some_and(|c| c.is_ascii_uppercase());
        let rest_is_plain =
            characters.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if !(first_is_letter && rest_is_plain) {
            return Err(SecretError::NameBadForm);
        }
        Ok(Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SecretName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
#[path = "secret_name.tests.rs"]
mod tests;
