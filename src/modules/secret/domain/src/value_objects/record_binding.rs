// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{SecretId, SecretName, SecretVersion};

const BINDING_PREFIX: &str = "office-harness/secret/v1";
const SEPARATOR: char = '\0';

/// What a sealed record is bound to: the secret's id, its name, and its
/// version. The sealer authenticates these bytes with the record, so a
/// record moved to another secret, or an old version put back after a
/// rotation, fails to open (contract, review L1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordBinding {
    pub id: SecretId,
    pub name: SecretName,
    pub version: SecretVersion,
}

impl RecordBinding {
    /// The authenticated data. Id, name, and version cannot hold the
    /// separator, so two bindings never give the same bytes.
    #[must_use]
    pub fn authenticated_data(&self) -> Vec<u8> {
        format!(
            "{BINDING_PREFIX}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{}",
            self.id,
            self.name,
            self.version.number()
        )
        .into_bytes()
    }
}

#[cfg(test)]
#[path = "record_binding.tests.rs"]
mod tests;
