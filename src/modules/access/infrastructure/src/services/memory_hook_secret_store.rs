// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use access_application::common::errors::AccessError;
use access_application::common::interfaces::HookSecretStore;
use access_domain::value_objects::OfficeToken;
use kernel::AgentId;

use super::random_bytes::random_bytes;

struct HookSecret {
    agent_id: AgentId,
    token: OfficeToken,
}

/// The status-hook secrets of the running sessions, in memory only (0024).
pub struct MemoryHookSecretStore {
    secrets: Mutex<HashMap<String, HookSecret>>,
    /// Compared with a presented secret when the session is unknown, so an
    /// unknown session takes the same work as a wrong secret.
    decoy: OfficeToken,
}

impl MemoryHookSecretStore {
    /// # Errors
    /// `Randomness` when the system has no random bytes.
    pub fn new() -> Result<Self, AccessError> {
        Ok(Self {
            secrets: Mutex::new(HashMap::new()),
            decoy: OfficeToken::from_random_bytes(random_bytes()?),
        })
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, HookSecret>> {
        // A panic elsewhere leaves the map itself consistent.
        self.secrets.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Compares two texts in time that depends on their length only.
fn same_text(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

impl HookSecretStore for MemoryHookSecretStore {
    fn issue(&self, agent_id: AgentId, session: &str) -> Result<OfficeToken, AccessError> {
        let token = OfficeToken::from_random_bytes(random_bytes()?);
        self.lock().insert(
            session.to_owned(),
            HookSecret {
                agent_id,
                token: token.clone(),
            },
        );
        Ok(token)
    }

    fn authorize(&self, session: &str, presented: &str) -> Option<AgentId> {
        let secrets = self.lock();
        let found = secrets.get(session);
        let expected = found.map_or(&self.decoy, |secret| &secret.token);
        let matches = same_text(expected.expose_secret(), presented);
        found.filter(|_| matches).map(|secret| secret.agent_id)
    }

    fn revoke_session(&self, session: &str) {
        self.lock().remove(session);
    }

    fn revoke_agent(&self, agent_id: AgentId) {
        self.lock().retain(|_, secret| secret.agent_id != agent_id);
    }
}

#[cfg(test)]
#[path = "memory_hook_secret_store.tests.rs"]
mod tests;
