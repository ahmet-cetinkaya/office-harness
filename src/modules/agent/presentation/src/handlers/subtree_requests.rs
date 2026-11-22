// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::commands::delete_agent::{DeleteWorktrees, WorktreeOutcome};
use kernel::AgentId;
use wire::{ErrorCode, Request};

/// The answer to one subtree stop or delete: the affected agents in the
/// order of the actions, or the failure mapped to a wire code.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub enum SubtreeResult {
    /// One entry per stopped agent, the requested agent last (FR-007).
    Stopped(Vec<AgentId>),
    /// One entry per deleted agent, deepest first and the requested agent
    /// last (FR-006), and what happened to their worktrees (0010 FR-007).
    Deleted {
        agents: Vec<AgentId>,
        worktrees: WorktreeOutcome,
    },
    /// The action ran on no agent: the wire code and message to answer.
    Failed { code: ErrorCode, message: String },
}

/// The hierarchy's subtree stop and delete, as the host bridges them for
/// `agent.stop` and `agent.delete`. The request is the wire request the
/// agent handlers received, so the fields stay the agent protocol's own;
/// no hierarchy type crosses this port.
pub trait SubtreeRequests: Send + Sync {
    /// Stops one agent or, when the request asks for it, its whole subtree
    /// (FR-007).
    fn stop_subtree(&self, request: &Request, agent_id: AgentId) -> SubtreeResult;

    /// Deletes one agent or, when the request confirms the fresh descendant
    /// count, its whole subtree (FR-006).
    /// `worktrees` is the request's own choice, already read, for every
    /// deleted agent (0010).
    fn delete_subtree(
        &self,
        request: &Request,
        agent_id: AgentId,
        worktrees: &DeleteWorktrees,
    ) -> SubtreeResult;
}
