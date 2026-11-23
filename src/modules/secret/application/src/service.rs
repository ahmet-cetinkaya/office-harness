// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kernel::AgentId;
use secret_domain::{
    Custody, MAX_SECRETS, RecordBinding, SecretId, SecretName, SecretValue, SecretVersion, StoreKey,
};

use crate::errors::SecretServiceError;
use crate::models::{AvailableSecret, GrantSummary, ReadAnswer, SecretSummary, StoreState};
use crate::ports::{RecordSealer, StoreKeyVault, VaultError};
use crate::provision::{load_store_key, provision_store_key};
use crate::store_ports::{
    AgentDirectory, SecretClock, SecretStore, StoreKeyGenerator, StoredGrant, StoredSecret,
};

type Result<T> = std::result::Result<T, SecretServiceError>;

/// Everything the service needs from the outside.
pub struct SecretPorts {
    pub store: Arc<dyn SecretStore>,
    pub keyring: Arc<dyn StoreKeyVault>,
    pub key_file: Arc<dyn StoreKeyVault>,
    pub generator: Arc<dyn StoreKeyGenerator>,
    pub sealer: Arc<dyn RecordSealer>,
    pub agents: Arc<dyn AgentDirectory>,
    pub clock: Arc<dyn SecretClock>,
}

/// The secret requests: the person's save, delete, grant, revoke, list, and
/// reset, and an agent's list and read. Writes run one at a time, so two saves
/// of one name end as two versions and never lose a value.
pub struct SecretService {
    ports: SecretPorts,
    write_lock: Mutex<()>,
}

impl SecretService {
    #[must_use]
    pub fn new(ports: SecretPorts) -> Self {
        Self {
            ports,
            write_lock: Mutex::new(()),
        }
    }

    /// # Errors
    /// `Storage` when the custody mark cannot be read.
    pub fn state(&self) -> Result<StoreState> {
        let Some(custody) = self.ports.store.custody()? else {
            return Ok(StoreState::Empty);
        };
        Ok(match self.open_key(custody) {
            Ok(_) => StoreState::Ready(custody),
            Err(SecretServiceError::Locked(reason)) => StoreState::Locked(custody, reason),
            Err(other) => return Err(other),
        })
    }

    /// The store state and every secret, ordered by name. A secret is marked
    /// unreadable when the store is ready and its record does not open.
    ///
    /// # Errors
    /// `Storage`.
    pub fn list(&self) -> Result<(StoreState, Vec<SecretSummary>)> {
        let state = self.state()?;
        let key = match state {
            StoreState::Ready(custody) => Some(self.open_key(custody)?),
            _ => None,
        };
        let summaries = self
            .ports
            .store
            .list()?
            .iter()
            .map(|stored| {
                let unreadable = key.as_ref().is_some_and(|key| !self.opens(key, stored));
                summarize(stored, unreadable)
            })
            .collect();
        Ok((state, summaries))
    }

    /// Saves a new secret, or replaces the value of an existing one (FR-001,
    /// FR-008). The first save creates the store key and chooses its custody.
    ///
    /// # Errors
    /// `TooManySecrets`, `Locked`, or `Storage`.
    pub fn save(&self, name: SecretName, value: &SecretValue) -> Result<SecretSummary> {
        let _write = self.write_guard();
        let key = self.key_for_save()?;
        let stored = match self.ports.store.find(&name)? {
            None => self.insert_new(&key, name, value)?,
            Some(existing) => self.replace_value(&key, existing, value)?,
        };
        Ok(summarize(&stored, false))
    }

    /// # Errors
    /// `NoSuchSecret` or `Storage`.
    pub fn delete(&self, name: &SecretName) -> Result<()> {
        let _write = self.write_guard();
        let stored = self.find_for_person(name)?;
        Ok(self.ports.store.delete(&stored.id)?)
    }

    /// # Errors
    /// `NoSuchSecret`, `NoSuchAgent`, `AlreadyGranted`, or `Storage`.
    pub fn grant(&self, name: &SecretName, agent: &AgentId) -> Result<SecretSummary> {
        let _write = self.write_guard();
        let stored = self.find_for_person(name)?;
        if !self.ports.agents.exists(agent)? {
            return Err(SecretServiceError::NoSuchAgent {
                agent_id: agent.to_string(),
            });
        }
        if !self.ports.store.add_grant(&stored.id, agent)? {
            return Err(SecretServiceError::AlreadyGranted);
        }
        self.summary_of(name)
    }

    /// # Errors
    /// `NoSuchSecret`, `NotGranted`, or `Storage`.
    pub fn revoke(&self, name: &SecretName, agent: &AgentId) -> Result<SecretSummary> {
        let _write = self.write_guard();
        let stored = self.find_for_person(name)?;
        if !self.ports.store.remove_grant(&stored.id, agent)? {
            return Err(SecretServiceError::NotGranted);
        }
        self.summary_of(name)
    }

    /// The secrets the agent may read, ordered by name; no value (FR-006).
    ///
    /// # Errors
    /// `Storage`.
    pub fn available(&self, agent: &AgentId) -> Result<Vec<AvailableSecret>> {
        Ok(self
            .ports
            .store
            .list()?
            .into_iter()
            .filter(|stored| holds_grant(stored, agent))
            .map(|stored| AvailableSecret {
                name: stored.name,
                version: stored.version,
            })
            .collect())
    }

    /// The current value of a granted secret (FR-006, FR-007, FR-019).
    ///
    /// # Errors
    /// `NotAvailable` for a missing, deleted, or ungranted secret, `Locked`,
    /// `CannotRead`, or `Storage`.
    pub fn read(&self, agent: &AgentId, name: &SecretName) -> Result<ReadAnswer> {
        let not_available = || SecretServiceError::NotAvailable {
            name: name.to_string(),
        };
        let stored = self
            .ports
            .store
            .find(name)?
            .filter(|stored| holds_grant(stored, agent))
            .ok_or_else(not_available)?;
        let custody = self.ports.store.custody()?.ok_or_else(not_available)?;
        let key = self.open_key(custody)?;
        let value = self
            .ports
            .sealer
            .open(&key, &binding_of(&stored), &stored.sealed)
            .map_err(|_| SecretServiceError::CannotRead {
                name: name.to_string(),
            })?;
        self.ports
            .store
            .record_read(&stored.id, agent, stored.version, self.ports.clock.now())?;
        Ok(ReadAnswer {
            name: stored.name,
            value,
            version: stored.version,
        })
    }

    /// Removes every secret, grant, and the store key (FR-015). The caller
    /// passes the number of secrets the person saw in the confirmation.
    ///
    /// # Errors
    /// `ConfirmCountDiffers` or `Storage`.
    pub fn reset(&self, confirmed_count: usize) -> Result<usize> {
        let _write = self.write_guard();
        let current = self.ports.store.list()?.len();
        if current != confirmed_count {
            return Err(SecretServiceError::ConfirmCountDiffers { current });
        }
        let custody = self.ports.store.custody()?;
        let removed = self.ports.store.reset()?;
        if let Some(custody) = custody {
            // The secrets are gone either way; a key that cannot be removed
            // now is of no use to anyone.
            let _ = self.vault_for(custody).remove();
        }
        Ok(removed)
    }

    /// Every secret that opens, with its value, for the redaction (FR-012).
    /// A locked or empty store gives none.
    ///
    /// # Errors
    /// `Storage`.
    pub fn open_all(&self) -> Result<Vec<(SecretName, SecretValue)>> {
        let StoreState::Ready(custody) = self.state()? else {
            return Ok(Vec::new());
        };
        let key = self.open_key(custody)?;
        let mut values = Vec::new();
        for stored in self.ports.store.list()? {
            if let Ok(value) = self
                .ports
                .sealer
                .open(&key, &binding_of(&stored), &stored.sealed)
            {
                values.push((stored.name, value));
            }
        }
        Ok(values)
    }

    /// Removes the grants of every agent that no longer exists (FR-009). A
    /// delete of one agent, a delete of a subtree, and a start after the
    /// agents were deleted while the server was down all take this path.
    ///
    /// Returns whether a grant was removed.
    ///
    /// # Errors
    /// `Storage`.
    pub fn forget_deleted_agents(&self) -> Result<bool> {
        let _write = self.write_guard();
        let mut removed_any = false;
        let mut checked = Vec::new();
        for stored in self.ports.store.list()? {
            for grant in stored.grants {
                if checked.contains(&grant.agent_id) {
                    continue;
                }
                checked.push(grant.agent_id);
                if !self.ports.agents.exists(&grant.agent_id)? {
                    self.ports.store.remove_grants_of_agent(&grant.agent_id)?;
                    removed_any = true;
                }
            }
        }
        Ok(removed_any)
    }

    fn write_guard(&self) -> MutexGuard<'_, ()> {
        self.write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn vault_for(&self, custody: Custody) -> &dyn StoreKeyVault {
        match custody {
            Custody::OsKeyring => &*self.ports.keyring,
            Custody::KeyFile => &*self.ports.key_file,
        }
    }

    fn open_key(&self, custody: Custody) -> Result<StoreKey> {
        load_store_key(self.vault_for(custody)).map_err(|error| match error {
            VaultError::Locked(reason) => SecretServiceError::Locked(reason),
            VaultError::Failed(message) => SecretServiceError::Storage {
                message,
                is_disk_failure: false,
            },
        })
    }

    /// The store key for a save; the first save makes it and keeps the custody.
    fn key_for_save(&self) -> Result<StoreKey> {
        if let Some(custody) = self.ports.store.custody()? {
            return self.open_key(custody);
        }
        let key =
            self.ports
                .generator
                .generate()
                .map_err(|message| SecretServiceError::Storage {
                    message,
                    is_disk_failure: false,
                })?;
        let custody = provision_store_key(&*self.ports.keyring, &*self.ports.key_file, &key)
            .map_err(|error| SecretServiceError::Storage {
                message: error.to_string(),
                is_disk_failure: false,
            })?;
        self.ports.store.set_custody(custody)?;
        Ok(key)
    }

    fn insert_new(
        &self,
        key: &StoreKey,
        name: SecretName,
        value: &SecretValue,
    ) -> Result<StoredSecret> {
        if self.ports.store.list()?.len() >= MAX_SECRETS {
            return Err(SecretServiceError::TooManySecrets { max: MAX_SECRETS });
        }
        let now = self.ports.clock.now();
        let id = SecretId::generate();
        let version = SecretVersion::first();
        let binding = RecordBinding {
            id: id.clone(),
            name: name.clone(),
            version,
        };
        let stored = StoredSecret {
            id,
            name,
            version,
            sealed: self.seal(key, &binding, value)?,
            created_at: now,
            updated_at: now,
            grants: Vec::new(),
        };
        self.ports.store.insert(&stored)?;
        Ok(stored)
    }

    fn replace_value(
        &self,
        key: &StoreKey,
        mut stored: StoredSecret,
        value: &SecretValue,
    ) -> Result<StoredSecret> {
        stored.version = stored.version.next();
        stored.updated_at = self.ports.clock.now();
        stored.sealed = self.seal(key, &binding_of(&stored), value)?;
        self.ports.store.replace(
            &stored.id,
            stored.version,
            &stored.sealed,
            stored.updated_at,
        )?;
        Ok(stored)
    }

    fn seal(
        &self,
        key: &StoreKey,
        binding: &RecordBinding,
        value: &SecretValue,
    ) -> Result<secret_domain::SealedRecord> {
        self.ports
            .sealer
            .seal(key, binding, value)
            .map_err(|error| SecretServiceError::Storage {
                message: error.to_string(),
                is_disk_failure: false,
            })
    }

    fn opens(&self, key: &StoreKey, stored: &StoredSecret) -> bool {
        self.ports
            .sealer
            .open(key, &binding_of(stored), &stored.sealed)
            .is_ok()
    }

    fn find_for_person(&self, name: &SecretName) -> Result<StoredSecret> {
        self.ports
            .store
            .find(name)?
            .ok_or_else(|| SecretServiceError::NoSuchSecret {
                name: name.to_string(),
            })
    }

    fn summary_of(&self, name: &SecretName) -> Result<SecretSummary> {
        Ok(summarize(&self.find_for_person(name)?, false))
    }
}

fn binding_of(stored: &StoredSecret) -> RecordBinding {
    RecordBinding {
        id: stored.id.clone(),
        name: stored.name.clone(),
        version: stored.version,
    }
}

fn holds_grant(stored: &StoredSecret, agent: &AgentId) -> bool {
    stored.grants.iter().any(|grant| grant.agent_id == *agent)
}

fn summarize(stored: &StoredSecret, unreadable: bool) -> SecretSummary {
    SecretSummary {
        name: stored.name.clone(),
        version: stored.version,
        created_at: stored.created_at,
        updated_at: stored.updated_at,
        unreadable,
        grants: stored.grants.iter().map(grant_summary).collect(),
    }
}

fn grant_summary(grant: &StoredGrant) -> GrantSummary {
    GrantSummary {
        agent_id: grant.agent_id,
        last_read_version: grant.last_read_version,
        last_read_at: grant.last_read_at,
    }
}
