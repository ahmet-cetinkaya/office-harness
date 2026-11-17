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

/// What can go wrong when the hierarchy asks the agent module to create or
/// remove one agent on its behalf. The text is the agent module's own
/// message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatorError(pub String);

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
    /// Maps the failure to the hierarchy's storage error: the tree is fine;
    /// the agent module could not do its part.
    #[must_use]
    pub fn into_application(self) -> HierarchyApplicationError {
        HierarchyApplicationError::Storage
    }
}
