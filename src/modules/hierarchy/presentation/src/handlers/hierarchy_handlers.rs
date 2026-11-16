// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::{CreateTeamHandler, DeleteTeamHandler, PlaceAgentHandler, TreeQuery};
use wire::{Event, Request, Response};

/// The request prefixes this module answers.
pub const REQUEST_PREFIXES: [&str; 2] = ["team.", "hierarchy."];

/// Answers the team and hierarchy requests. The handlers arrive with T3; the
/// type exists so the dispatcher can register the prefixes from day one.
#[derive(Clone)]
pub struct HierarchyHandlers {
    #[allow(dead_code, reason = "wired in T3 with the request bodies")]
    tree: std::sync::Arc<TreeQuery>,
    #[allow(dead_code, reason = "wired in T3 with the request bodies")]
    create_team: CreateTeamHandler,
    #[allow(dead_code, reason = "wired in T3 with the request bodies")]
    delete_team: DeleteTeamHandler,
    #[allow(dead_code, reason = "wired in T3 with the request bodies")]
    place_agent: PlaceAgentHandler,
}

impl HierarchyHandlers {
    #[must_use]
    pub fn new(
        tree: std::sync::Arc<TreeQuery>,
        create_team: CreateTeamHandler,
        delete_team: DeleteTeamHandler,
        place_agent: PlaceAgentHandler,
    ) -> Self {
        Self { tree, create_team, delete_team, place_agent }
    }

    /// Answers one request; T3 fills the bodies.
    ///
    /// # Panics
    /// Never; unknown types get a `NotFound` response.
    pub fn handle(&self, request: &Request) -> Response {
        let _ = &self.map_error;
        Response::failure(
            request.id,
            wire::ErrorCode::NotFound,
            format!("no request of the type {}", request.kind),
        )
    }

    fn map_error(&self, error: HierarchyApplicationError) -> wire::ErrorCode {
        match error {
            HierarchyApplicationError::Invalid(_) => wire::ErrorCode::Invalid,
            HierarchyApplicationError::NotFound(_) => wire::ErrorCode::NotFound,
            HierarchyApplicationError::Conflict(_) => wire::ErrorCode::Conflict,
            HierarchyApplicationError::Storage => wire::ErrorCode::Internal,
        }
    }
}
