// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use agent_domain::entities::{AdditionsRecord, Agent};
use agent_domain::value_objects::{
    AgentCommand, DisplayName, MachineId, OfficeAdditions, ProjectId,
};
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::additions_input::AdditionsInput;
use crate::common::command_line::split_command;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AdditionsStore, AgentStore, Clock, ProjectStore};
use crate::common::lookup::require_project;
use crate::common::models::AgentAction;

#[derive(Debug, Clone)]
pub struct CreateAgentRequest {
    pub name: String,
    pub command: String,
    /// Empty or absent means the project folder.
    pub working_folder: Option<PathBuf>,
    pub project_id: Option<ProjectId>,
    /// Absent means no additions.
    pub additions: Option<AdditionsInput>,
}

/// Creates an agent that never ran, so it shows `stopped` (`agent.create`,
/// FR-012).
#[derive(Clone)]
pub struct CreateAgentHandler {
    agents: Arc<dyn AgentStore>,
    projects: Arc<dyn ProjectStore>,
    additions: Arc<dyn AdditionsStore>,
    clock: Arc<dyn Clock>,
    journal: ActionJournal,
    machine_id: MachineId,
}

impl CreateAgentHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        projects: Arc<dyn ProjectStore>,
        additions: Arc<dyn AdditionsStore>,
        clock: Arc<dyn Clock>,
        journal: ActionJournal,
        machine_id: MachineId,
    ) -> Self {
        Self {
            agents,
            projects,
            additions,
            clock,
            journal,
            machine_id,
        }
    }

    /// # Errors
    /// `Invalid` naming the field at fault, `NotFound` for an unknown project,
    /// and `Conflict` when another agent has the name, ignoring case. A
    /// malformed addition is `Invalid` and creates nothing.
    pub fn handle(&self, request: CreateAgentRequest) -> Result<Agent, ApplicationError> {
        let name = DisplayName::parse("agent name", &request.name)?;
        let command = AgentCommand::parse(&request.command)?;
        // A command that cannot be split would fail every start.
        split_command(&command)?;
        let additions = request.additions.unwrap_or_default().into_additions()?;
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
        self.save_additions_or_undo_insert(&agent, additions)?;
        self.journal.record(agent.id, AgentAction::Create, None)?;
        Ok(agent)
    }

    /// The agent and its additions are one create: when the additions cannot
    /// be saved, the agent is removed, so no half-made agent stays behind.
    fn save_additions_or_undo_insert(
        &self,
        agent: &Agent,
        additions: OfficeAdditions,
    ) -> Result<(), ApplicationError> {
        let record = AdditionsRecord {
            additions,
            updated_at: agent.created_at,
        };
        let Err(saving) = self.additions.save(agent.id, &record) else {
            return Ok(());
        };
        // The save failure is the one to report; a failed undo adds nothing
        // the user can act on.
        let _ = self.agents.delete(agent.id);
        Err(saving.into())
    }
}

#[cfg(test)]
#[path = "create_agent_handler.tests.rs"]
mod tests;
