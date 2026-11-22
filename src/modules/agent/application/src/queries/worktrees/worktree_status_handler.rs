// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::Agent;
use agent_domain::value_objects::AgentWorktree;
use kernel::AgentId;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, WorktreeManager, WorktreeStatus};
use crate::common::lookup::require_agent;

/// One agent's worktree and its state, for the delete question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeStatusView {
    pub agent: Agent,
    pub worktree: AgentWorktree,
    pub status: WorktreeStatus,
}

/// The state of the own worktrees a delete covers (`agent.worktree_status`,
/// 0010 FR-005, FR-006).
#[derive(Clone)]
pub struct WorktreeStatusHandler {
    agents: Arc<dyn AgentStore>,
    worktrees: Option<Arc<dyn WorktreeManager>>,
}

impl WorktreeStatusHandler {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>, worktrees: Option<Arc<dyn WorktreeManager>>) -> Self {
        Self { agents, worktrees }
    }

    /// One view per listed agent that has an own worktree, in the order
    /// given. Without a worktree service every worktree reads as not
    /// reachable, so the question says it will be kept.
    ///
    /// # Errors
    /// `NotFound` for an unknown agent; `Storage`.
    pub fn handle(
        &self,
        agent_ids: &[AgentId],
    ) -> Result<Vec<WorktreeStatusView>, ApplicationError> {
        let mut views = Vec::new();
        for agent_id in agent_ids {
            let agent = require_agent(&*self.agents, *agent_id)?;
            let Some(worktree) = agent.worktree.clone() else {
                continue;
            };
            let status = self.status(&worktree);
            views.push(WorktreeStatusView {
                agent,
                worktree,
                status,
            });
        }
        Ok(views)
    }

    fn status(&self, worktree: &AgentWorktree) -> WorktreeStatus {
        self.worktrees.as_ref().map_or_else(
            || WorktreeStatus::unreachable(worktree),
            |worktrees| worktrees.status(worktree),
        )
    }
}
