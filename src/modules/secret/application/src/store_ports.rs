// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{AgentId, Timestamp};
use secret_domain::{Custody, SealedRecord, SecretId, SecretName, SecretVersion, StoreKey};
use thiserror::Error;

/// The stored form of one secret: everything but the value, and the sealed
/// record of the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSecret {
    pub id: SecretId,
    pub name: SecretName,
    pub version: SecretVersion,
    pub sealed: SealedRecord,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub grants: Vec<StoredGrant>,
}

/// One agent's grant on one secret, with the last read it made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredGrant {
    pub agent_id: AgentId,
    pub last_read_version: Option<SecretVersion>,
    pub last_read_at: Option<Timestamp>,
}

/// A storage failure. `is_disk_failure` lets the host raise the storage alert.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("secret storage failed: {message}")]
pub struct StoreError {
    pub message: String,
    pub is_disk_failure: bool,
}

/// The secrets, grants, and custody mark in the office database.
pub trait SecretStore: Send + Sync {
    /// Every secret with its grants, ordered by name.
    ///
    /// # Errors
    /// `StoreError` when the store cannot be read.
    fn list(&self) -> Result<Vec<StoredSecret>, StoreError>;

    /// # Errors
    /// `StoreError` when the store cannot be read.
    fn find(&self, name: &SecretName) -> Result<Option<StoredSecret>, StoreError>;

    /// Stores a new secret; its `grants` are empty.
    ///
    /// # Errors
    /// `StoreError`; nothing is stored.
    fn insert(&self, secret: &StoredSecret) -> Result<(), StoreError>;

    /// Replaces the sealed record and the version in one step.
    ///
    /// # Errors
    /// `StoreError`; the old record stays.
    fn replace(
        &self,
        id: &SecretId,
        version: SecretVersion,
        sealed: &SealedRecord,
        at: Timestamp,
    ) -> Result<(), StoreError>;

    /// Removes the secret and its grants.
    ///
    /// # Errors
    /// `StoreError`.
    fn delete(&self, id: &SecretId) -> Result<(), StoreError>;

    /// `false` when the agent already held the grant.
    ///
    /// # Errors
    /// `StoreError`.
    fn add_grant(&self, id: &SecretId, agent: &AgentId) -> Result<bool, StoreError>;

    /// `false` when the agent held no grant.
    ///
    /// # Errors
    /// `StoreError`.
    fn remove_grant(&self, id: &SecretId, agent: &AgentId) -> Result<bool, StoreError>;

    /// Removes every grant of a deleted agent (FR-009).
    ///
    /// # Errors
    /// `StoreError`.
    fn remove_grants_of_agent(&self, agent: &AgentId) -> Result<(), StoreError>;

    /// Notes the version an agent read last, and when.
    ///
    /// # Errors
    /// `StoreError`.
    fn record_read(
        &self,
        id: &SecretId,
        agent: &AgentId,
        version: SecretVersion,
        at: Timestamp,
    ) -> Result<(), StoreError>;

    /// The custody chosen at the first save, or `None` before it.
    ///
    /// # Errors
    /// `StoreError`.
    fn custody(&self) -> Result<Option<Custody>, StoreError>;

    /// # Errors
    /// `StoreError`.
    fn set_custody(&self, custody: Custody) -> Result<(), StoreError>;

    /// Removes every secret, every grant, and the custody mark. Returns how
    /// many secrets went.
    ///
    /// # Errors
    /// `StoreError`; nothing is removed.
    fn reset(&self) -> Result<usize, StoreError>;
}

/// Makes a new store key.
pub trait StoreKeyGenerator: Send + Sync {
    /// # Errors
    /// The text of the OS random source's failure.
    fn generate(&self) -> Result<StoreKey, String>;
}

/// Tells whether an agent exists, for a grant.
pub trait AgentDirectory: Send + Sync {
    /// # Errors
    /// `StoreError` when the agents cannot be read.
    fn exists(&self, agent: &AgentId) -> Result<bool, StoreError>;
}

pub trait SecretClock: Send + Sync {
    fn now(&self) -> Timestamp;
}

impl From<StoreError> for crate::SecretServiceError {
    fn from(error: StoreError) -> Self {
        Self::Storage {
            message: error.message,
            is_disk_failure: error.is_disk_failure,
        }
    }
}
