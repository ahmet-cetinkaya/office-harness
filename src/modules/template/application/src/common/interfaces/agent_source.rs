// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;

use crate::common::errors::TemplateApplicationError;
use crate::common::models::AgentDefinition;

/// Reads an agent's definition. The host app implements it over the agent and
/// hierarchy modules, so this module depends on neither.
pub trait AgentSource: Send + Sync {
    /// # Errors
    /// `NotFound` for an unknown agent, `Storage` for a read failure.
    fn definition_of(&self, agent_id: AgentId)
    -> Result<AgentDefinition, TemplateApplicationError>;
}
