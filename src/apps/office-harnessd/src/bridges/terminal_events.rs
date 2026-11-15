// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use terminal_application::common::interfaces::TerminalEventSink;
use terminal_domain::events::TerminalEvent;
use tokio::sync::broadcast;

const EVENT_BACKLOG: usize = 256;

/// One terminal event and the agent whose terminal produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentTerminalEvent {
    pub agent_id: AgentId,
    pub event: TerminalEvent,
}

/// The terminal module's event sink. It fans the plain events out to whoever
/// subscribes; the agent bridge turns them into status changes.
#[derive(Clone)]
pub struct TerminalEvents {
    sender: broadcast::Sender<AgentTerminalEvent>,
}

impl TerminalEvents {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BACKLOG);
        Self { sender }
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AgentTerminalEvent> {
        self.sender.subscribe()
    }
}

impl Default for TerminalEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalEventSink for TerminalEvents {
    fn publish(&self, agent_id: AgentId, event: TerminalEvent) {
        // No subscriber is a normal state: nobody listens yet.
        let _ = self.sender.send(AgentTerminalEvent { agent_id, event });
    }
}
