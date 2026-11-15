// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::errors::{ApplicationError, TerminalHostError};
use crate::common::interfaces::{AgentStore, SettingsStore, TerminalHost};
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;

/// Deletes an agent (`agent.delete`, FR-012). A running agent is stopped
/// first. The action record keeps the entries of the deleted agent.
#[derive(Clone)]
pub struct DeleteAgentHandler {
    agents: Arc<dyn AgentStore>,
    settings: Arc<dyn SettingsStore>,
    terminals: Arc<dyn TerminalHost>,
    journal: ActionJournal,
}

impl DeleteAgentHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        settings: Arc<dyn SettingsStore>,
        terminals: Arc<dyn TerminalHost>,
        journal: ActionJournal,
    ) -> Self {
        Self {
            agents,
            settings,
            terminals,
            journal,
        }
    }

    /// # Errors
    /// `NotFound`; `Internal` when a running child cannot be ended, which
    /// keeps the agent.
    pub fn handle(&self, agent_id: AgentId) -> Result<(), ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        if agent.status.is_running() {
            self.end_session(agent_id)?;
        }
        self.agents.delete(agent_id)?;
        self.journal.record(agent_id, AgentAction::Delete, None)
    }

    fn end_session(&self, agent_id: AgentId) -> Result<(), ApplicationError> {
        let grace = Duration::from_millis(self.settings.load()?.stop_grace_ms);
        match self.terminals.stop(agent_id, grace) {
            // The child ended on its own a moment ago.
            Ok(()) | Err(TerminalHostError::NotRunning) => Ok(()),
            Err(error) => Err(ApplicationError::Internal(error.to_string())),
        }
    }
}

#[cfg(test)]
#[path = "delete_agent_handler.tests.rs"]
mod tests;
