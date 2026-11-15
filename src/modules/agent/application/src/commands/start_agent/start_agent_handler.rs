// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use agent_domain::entities::Agent;
use agent_domain::enums::StatusEvent;
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::command_line::split_command;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, ProjectStore, TerminalHost};
use crate::common::lookup::{require_agent, require_project};
use crate::common::models::{AgentAction, StartTerminal};

/// The agent after a start that was accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartOutcome {
    /// `starting`, or `failed` when the command could not start.
    pub agent: Agent,
    /// The text to show when the command could not start.
    pub failure: Option<String>,
}

/// Starts an agent (`agent.start`, FR-012, FR-014). A command that cannot
/// start fails this agent only; it is not an error of the request.
#[derive(Clone)]
pub struct StartAgentHandler {
    agents: Arc<dyn AgentStore>,
    projects: Arc<dyn ProjectStore>,
    terminals: Arc<dyn TerminalHost>,
    journal: ActionJournal,
}

impl StartAgentHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        projects: Arc<dyn ProjectStore>,
        terminals: Arc<dyn TerminalHost>,
        journal: ActionJournal,
    ) -> Self {
        Self {
            agents,
            projects,
            terminals,
            journal,
        }
    }

    /// # Errors
    /// `NotFound` for an unknown agent, `Invalid` naming the missing folder,
    /// and `Conflict` when the agent already runs. A refusal leaves the
    /// status as it was and is recorded as a failed start.
    pub fn handle(&self, agent_id: AgentId) -> Result<StartOutcome, ApplicationError> {
        let mut agent = require_agent(&*self.agents, agent_id)?;
        let outcome = self.begin(&mut agent);
        let failure = match &outcome {
            Ok(outcome) => outcome.failure.clone(),
            Err(error) => Some(error.to_string()),
        };
        self.journal
            .record(agent_id, AgentAction::Start, failure.as_deref())?;
        outcome
    }

    fn begin(&self, agent: &mut Agent) -> Result<StartOutcome, ApplicationError> {
        let working_folder = self.existing_working_folder(agent)?;
        let (program, arguments) = split_command(&agent.command)?;
        agent.apply(StatusEvent::Start).map_err(|_| {
            ApplicationError::Conflict(format!("agent {} is already running", agent.name))
        })?;
        self.agents.update_status(agent.id, agent.status)?;
        let started = self.terminals.start(&StartTerminal {
            agent_id: agent.id,
            program,
            arguments,
            working_folder,
        });
        let failure = match started {
            Ok(()) => None,
            Err(error) => Some(error.to_string()),
        };
        if failure.is_some() {
            agent.apply(StatusEvent::ExitedWithFailure)?;
            self.agents.update_status(agent.id, agent.status)?;
        }
        Ok(StartOutcome {
            agent: agent.clone(),
            failure,
        })
    }

    fn existing_working_folder(&self, agent: &Agent) -> Result<PathBuf, ApplicationError> {
        let (field, folder) = match (&agent.working_folder, agent.project_id) {
            (Some(folder), _) => ("working folder", folder.clone()),
            (None, Some(project_id)) => (
                "project folder",
                require_project(&*self.projects, project_id)?.folder,
            ),
            (None, None) => {
                return Err(agent_domain::errors::AgentError::NoWorkingFolderOrProject.into());
            }
        };
        if folder.is_dir() {
            return Ok(folder);
        }
        Err(ApplicationError::Invalid(format!(
            "the {field} does not exist: {}",
            folder.display()
        )))
    }
}

#[cfg(test)]
#[path = "start_agent_handler.tests.rs"]
mod tests;
