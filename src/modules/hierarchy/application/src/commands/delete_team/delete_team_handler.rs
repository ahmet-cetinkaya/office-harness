// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::entities::TeamId;

use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;

/// What `team.delete` carries.
#[derive(Debug, Clone, Copy)]
pub struct DeleteTeamRequest {
    pub team_id: TeamId,
    /// The client asked the counted question first; this must be true.
    pub confirm: bool,
}

/// Removes a team record and unplaces its members (FR-002). The agents keep
/// their rows; they leave the hierarchy. Only the user deletes.
pub struct DeleteTeamHandler {
    store: Arc<dyn HierarchyStore>,
}

impl DeleteTeamHandler {
    #[must_use]
    pub fn new(store: Arc<dyn HierarchyStore>) -> Self {
        Self { store }
    }

    /// # Errors
    /// `Invalid` when `confirm` is not true; `NotFound` when the team is
    /// gone; `Storage` when the write fails.
    pub fn handle(&self, request: DeleteTeamRequest) -> Result<(), HierarchyApplicationError> {
        if !request.confirm {
            return Err(HierarchyApplicationError::Invalid(
                hierarchy_domain::HierarchyError::Empty { field: "confirm" },
            ));
        }
        let snapshot = self.store.snapshot()?;
        if !snapshot.teams.iter().any(|team| team.id() == request.team_id) {
            return Err(HierarchyApplicationError::NotFound(
                "the team does not exist".to_owned(),
            ));
        }
        self.store.delete_team_and_unplace(request.team_id)
    }
}
