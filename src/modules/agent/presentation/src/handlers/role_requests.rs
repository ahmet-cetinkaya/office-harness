// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::commands::create_agent::{NewTemp, OwnWorktree};
use agent_application::common::additions_input::AdditionsInput;
use wire::ErrorCode;

/// One created agent, or the failure mapped to a wire code.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub enum RoleResult {
    /// The agent was created and placed; the wire shows its record.
    Created(String),
    /// The create ran on no agent: the wire code and message to answer.
    Failed { code: ErrorCode, message: String },
}

impl RoleResult {
    /// The constructor of a refusal, so a bridge never builds the enum by
    /// hand.
    pub fn failed(code: ErrorCode, message: String) -> Self {
        Self::Failed { code, message }
    }
}

/// The hierarchy's `agent.create` with a `role` object (0002, FR-003), as
/// the host bridges it for `agent.create`. The seed stays the agent
/// protocol's own shape, so no hierarchy type crosses this port; the bridge
/// holds the tree exclusion across the check and the create+place.
pub trait RoleRequests: Send + Sync {
    /// Creates the agent the seed describes and places it in the tree.
    fn create_role_agent(
        &self,
        seed: &RoleSeed,
        kind: &str,
        parent_agent_id: Option<String>,
        team_id: Option<String>,
    ) -> RoleResult;
}

/// One created agent's own fields, as `agent.create` parsed them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleSeed {
    pub name: String,
    pub command: String,
    pub working_folder: Option<String>,
    pub project_id: Option<String>,
    /// Absent means no additions.
    pub additions: Option<AdditionsInput>,
    /// The user's own worktree choice (0010); agents never send one (A7).
    pub worktree: Option<OwnWorktree>,
    /// The temp choice and its creator (0020); `None` creates no temp.
    pub temp: Option<NewTemp>,
}
