// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::AgentStore;
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;
use crate::common::session_launcher::{LaunchKind, SessionLauncher, StartOutcome};

/// Starts an agent (`agent.start`, FR-012, FR-014, FR-024): a new session.
#[derive(Clone)]
pub struct StartAgentHandler {
    agents: Arc<dyn AgentStore>,
    launcher: SessionLauncher,
    journal: ActionJournal,
}

impl StartAgentHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        launcher: SessionLauncher,
        journal: ActionJournal,
    ) -> Self {
        Self {
            agents,
            launcher,
            journal,
        }
    }

    /// # Errors
    /// `NotFound` for an unknown agent, `Invalid` naming the missing folder
    /// or the unsafe argument, and `Conflict` when the agent already runs. A
    /// refusal leaves the status as it was and is recorded as a failed start.
    pub fn handle(&self, agent_id: AgentId) -> Result<StartOutcome, ApplicationError> {
        let mut agent = require_agent(&*self.agents, agent_id)?;
        let outcome = self.launcher.launch(&mut agent, &LaunchKind::NewTask);
        self.journal
            .record_start(agent_id, AgentAction::Start, &outcome, None)?;
        outcome
    }
}

#[cfg(test)]
#[path = "start_agent_handler.tests.rs"]
mod tests;
