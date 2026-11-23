// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use uuid::Uuid;

/// The random id of one secret. It is bound into the sealed record, so a
/// record cannot move to another secret.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretId(String);

impl SecretId {
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Wraps a stored id. The store only holds ids that `generate` made.
    #[must_use]
    pub fn from_stored(text: String) -> Self {
        Self(text)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SecretId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
