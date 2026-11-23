// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_domain::entities::{AdditionsRecord, Agent};
use agent_domain::errors::AgentError;
use agent_domain::value_objects::{
    AgentCommand, AgentWorktree, DisplayName, MachineId, OfficeAdditions, ProjectId, Settings,
};
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::additions_input::AdditionsInput;
use crate::common::command_line::split_command;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{
    AdditionsStore, AgentStore, Clock, CreatedWorktree, ProjectStore, SettingsStore,
    WorktreeCreate, WorktreeManager,
};
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
    /// Present asks for the agent's own branch and worktree (0010).
    pub worktree: Option<OwnWorktree>,
}

/// The own worktree choice of a create (0010 FR-002). Absent fields take
/// the chosen folder's base and the free branch name of now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnWorktree {
    pub base: Option<String>,
    pub branch_name: Option<String>,
}

/// The text when no git worktree service is attached or none can be used.
pub const NEEDS_REPOSITORY: &str = "own worktree needs a git repository";

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
    /// Absent refuses every own-worktree create.
    worktrees: Option<Arc<dyn WorktreeManager>>,
    /// Absent takes the default settings (0021 D11).
    settings: Option<Arc<dyn SettingsStore>>,
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
            worktrees: None,
            settings: None,
        }
    }

    /// Attaches the git worktree service, so a create can ask for an own
    /// worktree (0010).
    #[must_use]
    pub fn with_worktrees(mut self, worktrees: Arc<dyn WorktreeManager>) -> Self {
        self.worktrees = Some(worktrees);
        self
    }

    /// Attaches the office settings, which say whether a new own worktree
    /// shares the installed packages (0021).
    #[must_use]
    pub fn with_settings(mut self, settings: Arc<dyn SettingsStore>) -> Self {
        self.settings = Some(settings);
        self
    }

    /// # Errors
    /// `Invalid` naming the field at fault, `NotFound` for an unknown project,
    /// and `Conflict` when another agent has the name, ignoring case. A
    /// malformed addition is `Invalid` and creates nothing. An own worktree
    /// that cannot be made creates nothing either: no agent, branch, or
    /// folder is left (0010 FR-004).
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
        let agent_id = AgentId::generate();
        let created = match &request.worktree {
            Some(own) => Some(self.make_worktree(
                agent_id,
                &name,
                working_folder.as_deref(),
                request.project_id,
                own,
            )?),
            None => None,
        };
        // A refused agent discards the worktree made for it.
        let agent = self.new_agent(
            agent_id,
            name,
            command,
            working_folder,
            request.project_id,
            created,
        )?;
        if let Err(error) = self.agents.insert(&agent) {
            self.discard_worktree(&agent);
            return Err(ApplicationError::from_insert(
                error,
                "agent name",
                agent.name.as_str(),
            ));
        }
        self.save_additions_or_undo_insert(&agent, additions)?;
        if let Err(recording) = self.journal.record(agent.id, AgentAction::Create, None) {
            // A create the record cannot hold is undone whole, so the role
            // route that never learns the id leaves no agent, branch, or
            // folder behind.
            self.undo_insert(&agent);
            return Err(recording);
        }
        Ok(agent)
    }

    /// The agent the create stores. With an own worktree the working folder
    /// is the worktree's copy of the chosen folder (FR-002).
    fn new_agent(
        &self,
        agent_id: AgentId,
        name: DisplayName,
        command: AgentCommand,
        working_folder: Option<PathBuf>,
        project_id: Option<ProjectId>,
        created: Option<CreatedWorktree>,
    ) -> Result<Agent, ApplicationError> {
        let Some(created) = created else {
            return Ok(Agent::new(
                agent_id,
                name,
                command,
                working_folder,
                project_id,
                self.machine_id,
                self.clock.now(),
            )?);
        };
        let made = Agent::new(
            agent_id,
            name,
            command,
            Some(created.working_folder),
            project_id,
            self.machine_id,
            self.clock.now(),
        );
        match made {
            Ok(mut agent) => {
                agent.worktree = Some(created.worktree);
                Ok(agent)
            }
            Err(error) => {
                self.discard(&created.worktree);
                Err(error.into())
            }
        }
    }

    /// Makes the branch and the worktree from the chosen folder, which is
    /// the working folder or else the project's folder.
    fn make_worktree(
        &self,
        agent_id: AgentId,
        name: &DisplayName,
        working_folder: Option<&Path>,
        project_id: Option<ProjectId>,
        own: &OwnWorktree,
    ) -> Result<CreatedWorktree, ApplicationError> {
        let Some(worktrees) = &self.worktrees else {
            return Err(ApplicationError::Invalid(NEEDS_REPOSITORY.to_owned()));
        };
        let chosen = match (working_folder, project_id) {
            (Some(folder), _) => folder.to_path_buf(),
            (None, Some(project_id)) => require_project(&*self.projects, project_id)?.folder,
            (None, None) => return Err(AgentError::NoWorkingFolderOrProject.into()),
        };
        if !chosen.is_absolute() {
            return Err(AgentError::RelativePath {
                field: "working folder",
            }
            .into());
        }
        Ok(worktrees.create(&WorktreeCreate {
            folder: &chosen,
            agent_id,
            agent_name: name.as_str(),
            base: own.base.as_deref(),
            branch_name: own.branch_name.as_deref(),
            link_dependencies: self.links_dependencies()?,
        })?)
    }

    fn links_dependencies(&self) -> Result<bool, ApplicationError> {
        let settings = match &self.settings {
            Some(store) => store.load()?,
            None => Settings::default(),
        };
        Ok(settings.worktree_dependency_links)
    }

    fn discard_worktree(&self, agent: &Agent) {
        if let Some(worktree) = &agent.worktree {
            self.discard(worktree);
        }
    }

    fn discard(&self, worktree: &AgentWorktree) {
        if let Some(worktrees) = &self.worktrees {
            worktrees.discard(worktree);
        }
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
        self.undo_insert(agent);
        Err(saving.into())
    }

    /// Removes a stored agent and its worktree. The failure that caused the
    /// undo is the one to report; a failed undo adds nothing the user can
    /// act on.
    fn undo_insert(&self, agent: &Agent) {
        let _ = self.agents.delete(agent.id);
        self.discard_worktree(agent);
    }
}

#[cfg(test)]
#[path = "create_agent_handler.tests.rs"]
mod tests;
