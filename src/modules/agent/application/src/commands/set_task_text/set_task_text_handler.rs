// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Agent;
use agent_domain::value_objects::{TaskSource, TaskText};
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, Clock};
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;

/// Sets, edits, or clears an agent's task text for the user
/// (`agent.task.set`, FR-003). The text only labels the agent; nothing is
/// written to its terminal (A2).
#[derive(Clone)]
pub struct SetTaskTextHandler {
    agents: Arc<dyn AgentStore>,
    clock: Arc<dyn Clock>,
    journal: ActionJournal,
}

impl SetTaskTextHandler {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>, clock: Arc<dyn Clock>, journal: ActionJournal) -> Self {
        Self {
            agents,
            clock,
            journal,
        }
    }

    /// Returns the agent with its new task. A blank text clears the task.
    ///
    /// # Errors
    /// `NotFound` for an unknown agent, and `Invalid` naming the limit when
    /// the text has more than 8,192 characters, so nothing is stored.
    pub fn handle(&self, agent_id: AgentId, text: &str) -> Result<Agent, ApplicationError> {
        let mut agent = require_agent(&*self.agents, agent_id)?;
        let task = TaskText::from_input(text, self.clock.now(), TaskSource::User)?;
        self.agents.update_task(agent_id, task.as_ref())?;
        agent.task = task;
        self.journal.record(agent_id, AgentAction::TaskText, None)?;
        Ok(agent)
    }
}

#[cfg(test)]
#[path = "set_task_text_handler.tests.rs"]
mod tests;
