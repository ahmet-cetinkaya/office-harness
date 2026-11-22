// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;

/// The current names of agents, for the tie-break of the queue order. The
/// host app bridges it to the agent module.
pub trait AgentNameLookup: Send + Sync {
    /// The agent's name; empty when the agent is unknown.
    fn name_of(&self, agent_id: AgentId) -> String;
}
