// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::AgentStore;
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;
use crate::common::session_ender::SessionEnder;
use crate::common::session_launcher::{LaunchKind, SessionLauncher, StartOutcome};

/// Starts a new task (`agent.new_task`, FR-024): it ends a running session
/// first, always starts a new CLI session, and clears the task text once
/// that session has started (FR-004). The client confirms with the
/// user before it sends the request.
#[derive(Clone)]
pub struct StartNewTaskHandler {
    agents: Arc<dyn AgentStore>,
    ender: SessionEnder,
    launcher: SessionLauncher,
    journal: ActionJournal,
}

impl StartNewTaskHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        ender: SessionEnder,
        launcher: SessionLauncher,
        journal: ActionJournal,
    ) -> Self {
        Self {
            agents,
            ender,
            launcher,
            journal,
        }
    }

    /// # Errors
    /// As `StartAgentHandler`, and `Internal` when the running child cannot
    /// be ended.
    pub fn handle(&self, agent_id: AgentId) -> Result<StartOutcome, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let outcome = self.begin(agent);
        let failure = match &outcome {
            Ok(outcome) => outcome.failure.clone(),
            Err(error) => Some(error.to_string()),
        };
        self.journal
            .record(agent_id, AgentAction::NewTask, failure.as_deref())?;
        outcome
    }

    fn begin(
        &self,
        mut agent: agent_domain::entities::Agent,
    ) -> Result<StartOutcome, ApplicationError> {
        // Prepared before the running session ends, so a refusal loses nothing.
        let prepared = self.launcher.prepare(&agent, &LaunchKind::NewTask)?;
        if agent.status.is_running() {
            self.ender.stop_for_restart(&mut agent)?;
        }
        let mut outcome = self
            .launcher
            .start(&mut agent, &LaunchKind::NewTask, prepared)?;
        // A failed launch keeps the task text: the agent never began the new
        // task (0011 D3).
        if outcome.failure.is_none() {
            self.agents.update_task(agent.id, None)?;
            outcome.agent.task = None;
        }
        Ok(outcome)
    }
}

#[cfg(test)]
#[path = "start_new_task_handler.tests.rs"]
mod tests;
