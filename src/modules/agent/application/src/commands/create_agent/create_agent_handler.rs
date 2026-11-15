// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use agent_domain::entities::Agent;
use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId, ProjectId};
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::command_line::split_command;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, Clock, ProjectStore};
use crate::common::lookup::require_project;
use crate::common::models::AgentAction;

#[derive(Debug, Clone)]
pub struct CreateAgentRequest {
    pub name: String,
    pub command: String,
    /// Empty or absent means the project folder.
    pub working_folder: Option<PathBuf>,
    pub project_id: Option<ProjectId>,
}

/// Creates an agent that never ran, so it shows `stopped` (`agent.create`,
/// FR-012).
#[derive(Clone)]
pub struct CreateAgentHandler {
    agents: Arc<dyn AgentStore>,
    projects: Arc<dyn ProjectStore>,
    clock: Arc<dyn Clock>,
    journal: ActionJournal,
    machine_id: MachineId,
}

impl CreateAgentHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        projects: Arc<dyn ProjectStore>,
        clock: Arc<dyn Clock>,
        journal: ActionJournal,
        machine_id: MachineId,
    ) -> Self {
        Self {
            agents,
            projects,
            clock,
            journal,
            machine_id,
        }
    }

    /// # Errors
    /// `Invalid` naming the field at fault, `NotFound` for an unknown project,
    /// and `Conflict` when another agent has the name, ignoring case.
    pub fn handle(&self, request: CreateAgentRequest) -> Result<Agent, ApplicationError> {
        let name = DisplayName::parse("agent name", &request.name)?;
        let command = AgentCommand::parse(&request.command)?;
        // A command that cannot be split would fail every start.
        split_command(&command)?;
        if let Some(project_id) = request.project_id {
            require_project(&*self.projects, project_id)?;
        }
        let working_folder = request
            .working_folder
            .filter(|folder| !folder.as_os_str().is_empty());
        let agent = Agent::new(
            AgentId::generate(),
            name,
            command,
            working_folder,
            request.project_id,
            self.machine_id,
            self.clock.now(),
        )?;
        self.agents.insert(&agent).map_err(|error| {
            ApplicationError::from_insert(error, "agent name", agent.name.as_str())
        })?;
        self.journal.record(agent.id, AgentAction::Create, None)?;
        Ok(agent)
    }
}

#[cfg(test)]
#[path = "create_agent_handler.tests.rs"]
mod tests;
