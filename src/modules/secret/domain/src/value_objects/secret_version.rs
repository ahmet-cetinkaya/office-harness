// SPDX-License-Identifier: AGPL-3.0-or-later

/// Starts at 1 for a new secret and grows by 1 at each replace (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecretVersion(u32);

impl SecretVersion {
    #[must_use]
    pub fn first() -> Self {
        Self(1)
    }

    /// Wraps a stored version.
    #[must_use]
    pub fn from_stored(number: u32) -> Self {
        Self(number)
    }

    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    #[must_use]
    pub fn number(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
#[path = "secret_version.tests.rs"]
mod tests;
