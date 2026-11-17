// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_presentation::{SubtreeRequests, SubtreeResult};
use hierarchy_application::commands::subtree::SubtreeHandler;
use hierarchy_application::common::errors::HierarchyApplicationError;
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
            .stop_subtree(subtree_request(request, agent_id))
        {
            Ok(stopped) => SubtreeResult::Stopped(stopped),
            Err(error) => failed(&error),
        }
    }

    fn delete_subtree(&self, request: &Request, agent_id: AgentId) -> SubtreeResult {
        match self
            .subtree
            .delete_subtree(subtree_request(request, agent_id))
        {
            Ok(deleted) => SubtreeResult::Deleted(deleted),
            Err(error) => failed(&error),
        }
    }
}

/// The wire code of a failed subtree action: the hierarchy's error kinds
/// mapped as the agent handlers mapped them before.
fn failed(error: &HierarchyApplicationError) -> SubtreeResult {
    let code = match error {
        HierarchyApplicationError::Invalid(_) => ErrorCode::Invalid,
        HierarchyApplicationError::NotFound(_) => ErrorCode::NotFound,
        HierarchyApplicationError::Conflict(_) => ErrorCode::Conflict,
        HierarchyApplicationError::Storage => ErrorCode::Internal,
    };
    SubtreeResult::Failed {
        code,
        message: error.to_string(),
    }
}

#[cfg(test)]
#[path = "subtree_requests.tests.rs"]
mod tests;
