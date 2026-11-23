// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;

/// Whether an agent exists in the office. The agent module is not a
/// dependency of this one, so the host answers.
pub trait TargetCheck: Send + Sync {
    fn exists(&self, agent_id: AgentId) -> bool;
}
