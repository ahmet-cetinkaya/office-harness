// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use terminal_domain::events::TerminalEvent;

/// Receives the plain events of every terminal. The host app implements it
/// and turns the events into agent status changes. `publish` is called from
/// the terminal's own threads and must not block for long.
pub trait TerminalEventSink: Send + Sync {
    fn publish(&self, agent_id: AgentId, event: TerminalEvent);
}
