// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::AdditionsRecord;
use agent_domain::value_objects::OfficeAdditions;
use kernel::AgentId;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AdditionsStore, AgentStore};
use crate::common::lookup::require_agent;
use crate::common::models::AdditionsView;

/// An agent's office additions (`agent.additions.get`, FR-022). An agent
/// that never had any shows empty additions.
#[derive(Clone)]
pub struct GetOfficeAdditionsHandler {
    agents: Arc<dyn AgentStore>,
    additions: Arc<dyn AdditionsStore>,
}

impl GetOfficeAdditionsHandler {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>, additions: Arc<dyn AdditionsStore>) -> Self {
        Self { agents, additions }
    }

    /// # Errors
    /// `NotFound` for an unknown agent, or `Storage`.
    pub fn handle(&self, agent_id: AgentId) -> Result<AdditionsView, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let record = self
            .additions
            .load(agent_id)?
            .unwrap_or_else(|| AdditionsRecord {
                additions: OfficeAdditions::default(),
                updated_at: agent.created_at,
            });
        Ok(AdditionsView {
            record,
            applies_next_session: agent.status.is_running(),
        })
    }
}

#[cfg(test)]
#[path = "get_office_additions_handler.tests.rs"]
mod tests;
