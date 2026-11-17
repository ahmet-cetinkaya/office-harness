// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use kernel::AgentId;
use terminal_application::common::interfaces::TerminalEventSink;
use terminal_application::common::models::RunId;
use terminal_domain::events::TerminalEvent;
use tokio::sync::broadcast;
use tokio::sync::watch;

const EVENT_BACKLOG: usize = 256;

/// One terminal event and the agent whose terminal produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentTerminalEvent {
    pub agent_id: AgentId,
    /// The start of the terminal that produced the event.
    pub run: RunId,
    pub event: TerminalEvent,
}

/// The last line of one agent, with the run that wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldLine {
    pub agent_id: AgentId,
    pub run: RunId,
    pub line: String,
}

/// The terminal module's event sink. It fans the plain events out to whoever
/// subscribes; the agent bridge turns them into status changes, a changed
/// last line becomes an `agent.line` event, and the agent gateway reads the
/// held lines for the snapshots it answers.
#[derive(Clone)]
pub struct TerminalEvents {
    sender: broadcast::Sender<AgentTerminalEvent>,
    lines: Arc<Mutex<HashMap<AgentId, HeldLine>>>,
    line_changed: watch::Sender<u64>,
}

impl TerminalEvents {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BACKLOG);
        let (line_changed, _) = watch::channel(0);
        Self {
            sender,
            lines: Arc::new(Mutex::new(HashMap::new())),
            line_changed,
        }
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AgentTerminalEvent> {
        self.sender.subscribe()
    }

    /// Fires once after each stored line changed. The line forwarder waits
    /// on this instead of polling.
    #[must_use]
    pub fn subscribe_lines(&self) -> watch::Receiver<u64> {
        self.line_changed.subscribe()
    }

    /// The last line the agent's current run reported, or `None` when it has
    /// no line, or when the line belongs to an earlier run.
    #[must_use]
    pub fn last_line(&self, agent_id: AgentId, run: RunId) -> Option<String> {
        let lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
        lines
            .get(&agent_id)
            .filter(|held| held.run == run)
            .map(|held| held.line.clone())
    }

    /// The current line of every agent that holds one, in no order.
    #[must_use]
    pub fn held_lines(&self) -> Vec<HeldLine> {
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .cloned()
            .collect()
    }

    /// Forgets the lines of agents that no longer exist.
    pub fn retain_agents(&self, keep: impl Fn(AgentId) -> bool) {
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|agent_id, _| keep(*agent_id));
    }
}

impl Default for TerminalEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalEventSink for TerminalEvents {
    fn publish(&self, agent_id: AgentId, run: RunId, event: TerminalEvent) {
        // No subscriber is a normal state: nobody listens yet.
        let _ = self.sender.send(AgentTerminalEvent {
            agent_id,
            run,
            event,
        });
    }

    fn publish_line(&self, agent_id: AgentId, run: RunId, line: String) {
        // A stale run's line never overwrites the current one: a restart
        // clears the panel's text until the new child writes.
        {
            let mut lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
            if lines.get(&agent_id).is_some_and(|held| held.run >= run) {
                return;
            }
            lines.insert(
                agent_id,
                HeldLine {
                    agent_id,
                    run,
                    line,
                },
            );
        }
        // A receiver that dropped the send still holds a working value; the
        // send failing only means nobody forwards lines right now.
        let _ = self.line_changed.send_modified(|stamp| *stamp += 1);
    }
}
