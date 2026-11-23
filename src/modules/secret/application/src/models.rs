// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{AgentId, Timestamp};
use secret_domain::{Custody, LockedReason, SecretName, SecretValue, SecretVersion};

/// The state of the store (contract: `store`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreState {
    /// No secret was ever saved.
    Empty,
    Ready(Custody),
    Locked(Custody, LockedReason),
}

/// What the person sees of one secret: never the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSummary {
    pub name: SecretName,
    pub version: SecretVersion,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    /// The sealed record does not open although the store is ready.
    pub unreadable: bool,
    pub grants: Vec<GrantSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantSummary {
    pub agent_id: AgentId,
    pub last_read_version: Option<SecretVersion>,
    pub last_read_at: Option<Timestamp>,
}

/// One secret an agent may read: no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableSecret {
    pub name: SecretName,
    pub version: SecretVersion,
}

/// The answer of a granted read. The value prints as `[secret]`.
#[derive(Debug)]
pub struct ReadAnswer {
    pub name: SecretName,
    pub value: SecretValue,
    pub version: SecretVersion,
}
