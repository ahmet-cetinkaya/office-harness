// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use access_application::common::errors::AccessError;
use access_application::common::interfaces::{AgentCallStore, AgentCredential};
use access_domain::value_objects::OfficeToken;
use kernel::AgentId;

use super::random_bytes::random_bytes;

/// The call credentials of the running agents, in memory only. A server
/// restart ends every session, so a credential never outlives its server.
#[derive(Default)]
pub struct MemoryAgentCallStore {
    credentials: Mutex<HashMap<AgentId, AgentCredential>>,
}

impl MemoryAgentCallStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<AgentId, AgentCredential>> {
        // A panic elsewhere leaves the map itself consistent.
        self.credentials
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// A random token that no agent holds, for the proof check of an unknown
/// agent.
///
/// # Errors
/// `Randomness` when the system has no random bytes.
pub fn random_decoy_token() -> Result<OfficeToken, AccessError> {
    Ok(OfficeToken::from_random_bytes(random_bytes()?))
}

impl AgentCallStore for MemoryAgentCallStore {
    fn issue(&self, agent_id: AgentId, session: &str) -> Result<AgentCredential, AccessError> {
        let credential = AgentCredential {
            token: OfficeToken::from_random_bytes(random_bytes()?),
            session: session.to_owned(),
        };
        self.lock().insert(agent_id, credential.clone());
        Ok(credential)
    }

    fn current(&self, agent_id: AgentId) -> Option<AgentCredential> {
        self.lock().get(&agent_id).cloned()
    }

    fn revoke_session(&self, agent_id: AgentId, session: &str) {
        let mut credentials = self.lock();
        if credentials
            .get(&agent_id)
            .is_some_and(|credential| credential.session == session)
        {
            credentials.remove(&agent_id);
        }
    }

    fn revoke_agent(&self, agent_id: AgentId) {
        self.lock().remove(&agent_id);
    }
}

#[cfg(test)]
#[path = "memory_agent_call_store.tests.rs"]
mod tests;
