// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;

/// The last check before an agent goes to sleep (0018 D4). The sleep step
/// asks it under the status order, so the check and the stop are one step
/// with respect to every status change. The host implements it.
pub trait SleepGate: Send + Sync {
    /// Why the agent must stay awake, or `None` when nothing blocks the sleep.
    fn blocking_reason(&self, agent_id: AgentId) -> Option<String>;

    /// The agent is going to sleep: the host refuses directives and reports to
    /// it from now on.
    fn begin(&self, agent_id: AgentId);

    /// The sleep step is over, whether it worked or not. Pairs with `begin`.
    fn end(&self, agent_id: AgentId);
}
