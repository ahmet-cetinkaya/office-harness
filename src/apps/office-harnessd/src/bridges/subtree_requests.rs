// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::commands::delete_agent::{DeleteWorktrees, WorktreeOutcome};
use agent_presentation::{SubtreeRequests, SubtreeResult};
use hierarchy_application::commands::subtree::{SubtreeHandler, SubtreeRequest};
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::common::ports::{SubtreeWorktrees, WorktreeReport};
use hierarchy_presentation::subtree_request;
use kernel::AgentId;
use wire::{ErrorCode, Request};

/// Answers the agent module's subtree port with the hierarchy's subtree use
/// cases: it reads the agent wire request the way the hierarchy handlers do,
/// runs the stop or delete, and maps a failure to the wire code the agent
/// handlers answered with before the modules were separated (FR-006, FR-007).
pub struct SubtreeRequestsBridge {
    subtree: SubtreeHandler,
}

impl SubtreeRequestsBridge {
    #[must_use]
    pub fn new(subtree: SubtreeHandler) -> Self {
        Self { subtree }
    }
}

impl SubtreeRequests for SubtreeRequestsBridge {
    fn stop_subtree(&self, request: &Request, agent_id: AgentId) -> SubtreeResult {
        match self
            .subtree
            .stop_subtree(&subtree_request(request, agent_id))
        {
            Ok(stopped) => SubtreeResult::Stopped(stopped),
            Err(error) => failed(&error),
        }
    }

    fn delete_subtree(
        &self,
        request: &Request,
        agent_id: AgentId,
        worktrees: &DeleteWorktrees,
    ) -> SubtreeResult {
        let request = SubtreeRequest {
            worktrees: subtree_worktrees(worktrees),
            ..subtree_request(request, agent_id)
        };
        match self.subtree.delete_subtree(&request) {
            Ok(deletion) => SubtreeResult::Deleted {
                agents: deletion.deleted,
                worktrees: worktree_outcome(deletion.worktrees),
            },
            Err(error) => failed(&error),
        }
    }
}

/// The agent module's worktree choice as the hierarchy carries it. The
/// create undo never comes this way, so a discard reads as keep.
#[must_use]
pub fn subtree_worktrees(worktrees: &DeleteWorktrees) -> SubtreeWorktrees {
    match worktrees {
        DeleteWorktrees::Remove(confirmed) => SubtreeWorktrees {
            remove: true,
            confirmed_status: confirmed.clone(),
        },
        DeleteWorktrees::Keep | DeleteWorktrees::Discard => SubtreeWorktrees::default(),
    }
}

/// The hierarchy's carried choice back as the agent module's own.
#[must_use]
pub fn delete_worktrees(worktrees: &SubtreeWorktrees) -> DeleteWorktrees {
    if worktrees.remove {
        DeleteWorktrees::Remove(worktrees.confirmed_status.clone())
    } else {
        DeleteWorktrees::Keep
    }
}

/// The agent module's outcome as the hierarchy reports it.
#[must_use]
pub fn worktree_report(outcome: WorktreeOutcome) -> WorktreeReport {
    WorktreeReport {
        deleted_branches: outcome.deleted_branches,
        kept_branches: outcome.kept_branches,
        kept_folders: outcome.kept_folders,
    }
}

fn worktree_outcome(report: WorktreeReport) -> WorktreeOutcome {
    WorktreeOutcome {
        deleted_branches: report.deleted_branches,
        kept_branches: report.kept_branches,
        kept_folders: report.kept_folders,
    }
}

/// The wire code of a failed subtree action: the hierarchy's error kinds
/// mapped as the agent handlers mapped them before.
fn failed(error: &HierarchyApplicationError) -> SubtreeResult {
    let code =
        match error {
            HierarchyApplicationError::Invalid(_) | HierarchyApplicationError::InvalidInput(_) => {
                ErrorCode::Invalid
            }
            // The `*Input` variants carry an agent-module message, so they keep
            // the code of their kind — only the prefix is bare.
            HierarchyApplicationError::NotFoundInput(_)
            | HierarchyApplicationError::NotFound(_) => ErrorCode::NotFound,
            HierarchyApplicationError::ConflictInput(_)
            | HierarchyApplicationError::Conflict(_) => ErrorCode::Conflict,
            // A hold or a rate limit refused the action (0007).
            HierarchyApplicationError::Refused(_) => ErrorCode::Refused,
            HierarchyApplicationError::Storage(_) => ErrorCode::Internal,
        };
    SubtreeResult::Failed {
        code,
        message: error.to_string(),
    }
}

#[cfg(test)]
#[path = "subtree_requests.tests.rs"]
mod tests;
