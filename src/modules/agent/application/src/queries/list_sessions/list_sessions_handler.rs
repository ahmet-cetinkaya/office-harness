// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::AgentId;

use crate::common::continuation::{adapter_of, continuation_of};
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, CliAdapters, SessionStore};
use crate::common::lookup::require_agent;
use crate::common::models::SessionView;

/// The sessions of an agent, newest first, each with `can_continue` and the
/// reason when it is false (`session.list`, FR-025).
#[derive(Clone)]
pub struct ListSessionsHandler {
    agents: Arc<dyn AgentStore>,
    sessions: Arc<dyn SessionStore>,
    adapters: Arc<dyn CliAdapters>,
}

impl ListSessionsHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        sessions: Arc<dyn SessionStore>,
        adapters: Arc<dyn CliAdapters>,
    ) -> Self {
        Self {
            agents,
            sessions,
            adapters,
        }
    }

    /// # Errors
    /// `NotFound` for an unknown agent, or `Storage`.
    pub fn handle(&self, agent_id: AgentId) -> Result<Vec<SessionView>, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let adapter = adapter_of(&*self.adapters, &agent.command)?;
        let sessions = self.sessions.list_for_agent(agent_id)?;
        Ok(sessions
            .into_iter()
            .map(|session| SessionView {
                cannot_continue_reason: continuation_of(&session, &*adapter).err(),
                session,
            })
            .collect())
    }
}

#[cfg(test)]
#[path = "list_sessions_handler.tests.rs"]
mod tests;
