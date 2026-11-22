// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use kernel::AgentId;

use crate::common::errors::HierarchyApplicationError;

/// What can go wrong when the agent module stops or deletes one agent on the
/// hierarchy's behalf. The text is the agent module's own message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtreeActionError(pub String);

/// Stops one agent, as `agent.stop` does (FR-007).
pub trait SubtreeAgentEnder: Send + Sync {
    /// # Errors
    /// The agent module's failure when the agent cannot be stopped.
    fn stop(&self, agent_id: AgentId) -> Result<(), SubtreeActionError>;
}

/// The user's worktree choice of a delete (0010 FR-005), as the agent
/// protocol carried it. The hierarchy hands it unread to the agent module
/// for every deleted agent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubtreeWorktrees {
    /// Remove the worktrees; false keeps them.
    pub remove: bool,
    /// The status hash the user confirmed, per agent.
    pub confirmed_status: HashMap<AgentId, String>,
}

/// The branches and folders one or more deletes removed or kept (0010
/// FR-007).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeReport {
    pub deleted_branches: Vec<String>,
    pub kept_branches: Vec<String>,
    pub kept_folders: Vec<String>,
}

impl WorktreeReport {
    pub fn extend(&mut self, other: Self) {
        self.deleted_branches.extend(other.deleted_branches);
        self.kept_branches.extend(other.kept_branches);
        self.kept_folders.extend(other.kept_folders);
    }
}

/// Deletes one agent, as `agent.delete` does (FR-006).
pub trait SubtreeAgentRemover: Send + Sync {
    /// Checks, without changing anything, that the agent's delete would not
    /// be refused for a worktree that changed since the user confirmed it,
    /// so a subtree delete refuses before its first step (0010 D6).
    ///
    /// # Errors
    /// The agent module's refusal.
    fn check(
        &self,
        agent_id: AgentId,
        worktrees: &SubtreeWorktrees,
    ) -> Result<(), SubtreeActionError>;

    /// Stops the agent when the delete removes its worktree, then checks
    /// again, so no member writes after the last check (0010 D6).
    ///
    /// # Errors
    /// The agent module's refusal; the agent stays stopped.
    fn prepare(
        &self,
        agent_id: AgentId,
        worktrees: &SubtreeWorktrees,
    ) -> Result<(), SubtreeActionError>;

    /// # Errors
    /// The agent module's failure when the agent cannot be deleted.
    fn delete(
        &self,
        agent_id: AgentId,
        worktrees: &SubtreeWorktrees,
    ) -> Result<WorktreeReport, SubtreeActionError>;
}

impl SubtreeActionError {
    /// Maps the failure to the application error of a subtree action.
    #[must_use]
    pub fn into_application(self) -> HierarchyApplicationError {
        HierarchyApplicationError::Conflict(self.0)
    }
}
