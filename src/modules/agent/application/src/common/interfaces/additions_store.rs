// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::AdditionsRecord;
use kernel::AgentId;

use crate::common::errors::PortError;

/// The office additions of each agent, in office-harness's own storage
/// (FR-022). The CLI's own configuration is never touched.
pub trait AdditionsStore: Send + Sync {
    /// # Errors
    /// `Storage`.
    fn load(&self, agent_id: AgentId) -> Result<Option<AdditionsRecord>, PortError>;

    /// Replaces the agent's additions.
    ///
    /// # Errors
    /// `Storage`.
    fn save(&self, agent_id: AgentId, record: &AdditionsRecord) -> Result<(), PortError>;
}
