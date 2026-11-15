// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::AdditionsRecord;
use kernel::AgentId;

use crate::common::additions_input::AdditionsInput;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AdditionsStore, AgentStore, Clock};
use crate::common::lookup::require_agent;
use crate::common::models::AdditionsView;

/// Stores an agent's office additions (`agent.additions.set`, FR-022). A
/// running session keeps the values it started with; the next session gets
/// the new ones (FR-023).
#[derive(Clone)]
pub struct SetOfficeAdditionsHandler {
    agents: Arc<dyn AgentStore>,
    additions: Arc<dyn AdditionsStore>,
    clock: Arc<dyn Clock>,
}

impl SetOfficeAdditionsHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        additions: Arc<dyn AdditionsStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            agents,
            additions,
            clock,
        }
    }

    /// # Errors
    /// `NotFound` for an unknown agent, and `Invalid` naming the malformed
    /// entry, so nothing is stored.
    pub fn handle(
        &self,
        agent_id: AgentId,
        input: AdditionsInput,
    ) -> Result<AdditionsView, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let record = AdditionsRecord {
            additions: input.into_additions()?,
            updated_at: self.clock.now(),
        };
        self.additions.save(agent_id, &record)?;
        Ok(AdditionsView {
            record,
            applies_next_session: agent.status.is_running(),
        })
    }
}

#[cfg(test)]
#[path = "set_office_additions_handler.tests.rs"]
mod tests;
