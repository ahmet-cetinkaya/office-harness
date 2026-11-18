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
    pub env: Vec<(String, String)>,
    pub startup_prompt: Option<String>,
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
    /// # Errors
    /// The agent module's failure when the agent cannot be created.
    fn create(&self, seed: AgentSeed) -> Result<AgentId, CreatorError>;

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
            CreatorErrorKind::Internal => HierarchyApplicationError::Storage(self.message),
        }
    }
}
