// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use kernel::{AgentId, Timestamp};

use crate::enums::{AgentStatus, StatusEvent};
use crate::errors::AgentError;
use crate::services::next_status;
use crate::value_objects::{
    AgentCommand, AgentWorktree, DisplayName, MachineId, ProjectId, TaskText,
};

/// An agent definition and its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub id: AgentId,
    pub name: DisplayName,
    pub command: AgentCommand,
    /// Empty means the project folder.
    pub working_folder: Option<PathBuf>,
    pub project_id: Option<ProjectId>,
    pub machine_id: MachineId,
    pub status: AgentStatus,
    pub created_at: Timestamp,
    /// The last task the office gave the agent, if any (0011).
    pub task: Option<TaskText>,
    /// The agent's own branch and worktree, when it was created with one
    /// (0010). The working folder then lies inside the worktree.
    pub worktree: Option<AgentWorktree>,
}

impl Agent {
    /// A new agent that never ran, so it shows `stopped`.
    ///
    /// # Errors
    /// `RelativePath` when the working folder is not absolute, and
    /// `NoWorkingFolderOrProject` when the agent has neither.
    pub fn new(
        id: AgentId,
        name: DisplayName,
        command: AgentCommand,
        working_folder: Option<PathBuf>,
        project_id: Option<ProjectId>,
        machine_id: MachineId,
        created_at: Timestamp,
    ) -> Result<Self, AgentError> {
        if working_folder.is_none() && project_id.is_none() {
            return Err(AgentError::NoWorkingFolderOrProject);
        }
        if working_folder
            .as_ref()
            .is_some_and(|folder| !folder.is_absolute())
        {
            return Err(AgentError::RelativePath {
                field: "working folder",
            });
        }
        Ok(Self {
            id,
            name,
            command,
            working_folder,
            project_id,
            machine_id,
            status: AgentStatus::Stopped,
            created_at,
            task: None,
            worktree: None,
        })
    }

    /// Moves the agent along the status state machine.
    ///
    /// # Errors
    /// `InvalidTransition` when the event is not allowed in the current status.
    pub fn apply(&mut self, event: StatusEvent) -> Result<(), AgentError> {
        self.status = next_status(self.status, event)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "agent.tests.rs"]
mod tests;
