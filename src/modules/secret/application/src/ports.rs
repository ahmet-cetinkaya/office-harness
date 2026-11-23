// SPDX-License-Identifier: AGPL-3.0-or-later

use secret_domain::{Custody, LockedReason, RecordBinding, SealedRecord, SecretValue, StoreKey};
use thiserror::Error;

/// Why a vault cannot give or take the store key. No text holds the key.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VaultError {
    /// Sealed secrets may exist, and the key cannot be had now (FR-019).
    #[error("the secret store is locked: {0}")]
    Locked(LockedReason),
    #[error("the store key vault failed: {0}")]
    Failed(String),
}

/// One place that can keep the store key: the OS keyring or the key file.
pub trait StoreKeyVault: Send + Sync {
    fn custody(&self) -> Custody;

    /// `Ok(None)` when the vault answers and holds no key.
    ///
    /// # Errors
    /// `Locked` when the vault cannot answer now, or `Failed`.
    fn load(&self) -> Result<Option<StoreKey>, VaultError>;

    /// # Errors
    /// `Locked` or `Failed`; nothing is half stored.
    fn store(&self, key: &StoreKey) -> Result<(), VaultError>;

    /// Removes the key. A missing key is not an error.
    ///
    /// # Errors
    /// `Locked` or `Failed`.
    fn remove(&self) -> Result<(), VaultError>;
}

/// A record cannot be sealed or opened. The text holds no value and no detail
/// of the failure (contract: `internal`).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SealError {
    #[error("the secret cannot be sealed")]
    CannotSeal,
    #[error("the secret cannot be opened")]
    CannotOpen,
}

/// Seals and opens one value under the store key, bound to its record.
pub trait RecordSealer: Send + Sync {
    /// # Errors
    /// `CannotSeal`.
    fn seal(
        &self,
        key: &StoreKey,
        binding: &RecordBinding,
        value: &SecretValue,
    ) -> Result<SealedRecord, SealError>;

    /// # Errors
    /// `CannotOpen` when the record is damaged, moved, old, or sealed under
    /// another key.
    fn open(
        &self,
        key: &StoreKey,
        binding: &RecordBinding,
        record: &SealedRecord,
    ) -> Result<SecretValue, SealError>;
}
