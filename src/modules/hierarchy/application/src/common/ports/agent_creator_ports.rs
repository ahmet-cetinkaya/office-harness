// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use kernel::AgentId;
use kernel::ProjectId;

use crate::common::errors::HierarchyApplicationError;

/// Office additions as a client sent them, before the domain checks them.
/// The agent module's own `AdditionsInput` is not reachable from this crate,
/// so its shape is mirrored here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedAdditions {
    pub flags: Vec<String>,
    pub env: Vec<(String, Option<String>)>,
    pub startup_prompt: Option<String>,
}

/// The own worktree choice of a seeded create (0010), mirrored like the
/// additions. Absent fields take the folder's base and the free branch name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedWorktree {
    pub base: Option<String>,
    pub branch_name: Option<String>,
}

/// The temporary mark of a seeded create (0020): the agent module checks the
/// objective text and the cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempSeed {
    pub objective: String,
}

/// What creating an agent needs. This mirrors the agent module's
/// `CreateAgentRequest`: that type is public, but it lives in the agent
/// module's application crate, which the hierarchy does not depend on, so
/// the bootstrap bridges this seed to the real create use case.
#[derive(Debug, Clone)]
pub struct AgentSeed {
    pub name: String,
    pub command: String,
    /// Empty or absent means the project folder.
    pub working_folder: Option<PathBuf>,
    pub project_id: Option<ProjectId>,
    /// Absent means no additions.
    pub additions: Option<SeedAdditions>,
    /// Present asks for the agent's own branch and worktree (0010).
    pub worktree: Option<SeedWorktree>,
    /// Present makes the agent a temporary agent (0020).
    pub temp: Option<TempSeed>,
}

/// Why the agent module refused. The kinds follow its error contract, so a
/// seeded create answers with the code that `agent.create` answers with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorErrorKind {
    /// A field of the seed broke an agent-module rule.
    Invalid,
    /// A referenced entry (a project, an agent) does not exist.
    NotFound,
    /// Another agent already holds the name or the resource.
    Conflict,
    /// The office's own rule said no, as the temp cap does (0020 D3); it
    /// answers `refused`, so it counts toward no rate limit.
    Refused,
    /// The agent module could not reach its own storage.
    Internal,
}

/// What can go wrong when the hierarchy asks the agent module to create or
/// remove one agent on its behalf. The message is the agent module's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatorError {
    pub kind: CreatorErrorKind,
    pub message: String,
}

/// Creates one agent, as `agent.create` does, and removes it again when the
/// hierarchy cannot give it a place.
pub trait AgentCreator: Send + Sync {
    /// Every reason the agent module would refuse `seed`, without writing
    /// anything: the name, the command, and the additions rules. Empty when
    /// it would accept. A creator with no rules of its own accepts all.
    fn check(&self, _seed: &AgentSeed) -> Vec<String> {
        Vec::new()
    }

    /// # Errors
    /// The agent module's failure when the agent cannot be created.
    fn create(&self, seed: AgentSeed) -> Result<AgentId, CreatorError>;

    /// The undo of `create`: deletes the agent and everything its create
    /// made, its own worktree and branch included, without asking (the
    /// create undo mode, 0010 D5).
    ///
    /// # Errors
    /// The agent module's failure when the agent cannot be removed.
    fn remove(&self, agent_id: AgentId) -> Result<(), CreatorError>;
}

impl CreatorError {
    /// Maps the failure to the application error of the same kind, keeping
    /// the agent module's message: the tree is fine; the agent module
    /// refused or failed.
    #[must_use]
    pub fn into_application(self) -> HierarchyApplicationError {
        match self.kind {
            CreatorErrorKind::Invalid => HierarchyApplicationError::InvalidInput(self.message),
            CreatorErrorKind::NotFound => HierarchyApplicationError::NotFoundInput(self.message),
            CreatorErrorKind::Conflict => HierarchyApplicationError::ConflictInput(self.message),
            CreatorErrorKind::Refused => HierarchyApplicationError::Refused(self.message),
            CreatorErrorKind::Internal => HierarchyApplicationError::Storage(self.message),
        }
    }
}
