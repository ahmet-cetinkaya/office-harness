// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use agent_presentation::agent_line_event;
use kernel::AgentId;
use terminal_application::common::models::RunId;

use super::terminal_events::{HeldLine, TerminalEvents};
use crate::server::EventHub;

/// The event for a line this forwarder has not sent yet. `sent` remembers the
/// run and text already on the wire per agent, so a wake that a second change
/// caused does not resend the first line, and a new run's line is never
/// mistaken for the old run's text.
fn event_if_new(
    held: &HeldLine,
    sent: &mut HashMap<AgentId, (RunId, String)>,
) -> Option<wire::Event> {
    if sent
        .get(&held.agent_id)
        .is_some_and(|(run, line)| *run == held.run && line == &held.line)
    {
        return None;
    }
    sent.insert(held.agent_id, (held.run, held.line.clone()));
    Some(agent_line_event(held.agent_id, &held.line))
}

/// Forwards every changed last line to the clients as `agent.line`. The
/// terminal's monitor throttles to one line per agent per second; this only
/// wakes on a change and sends what is not on the wire yet.
pub struct AgentLineBridge {
    lines: TerminalEvents,
    hub: EventHub,
    sent: HashMap<AgentId, (RunId, String)>,
}

impl AgentLineBridge {
    #[must_use]
    pub fn new(lines: TerminalEvents, hub: EventHub) -> Self {
        Self {
            lines,
            hub,
            sent: HashMap::new(),
        }
    }

    /// Runs until the source is dropped.
    pub fn spawn(mut self, mut changes: tokio::sync::watch::Receiver<u64>) {
        tokio::spawn(async move {
            loop {
                if changes.changed().await.is_err() {
                    break;
                }
                self.forward_new_lines();
            }
        });
    }

    /// Publishes an event for every held line that is not on the wire yet.
    fn forward_new_lines(&mut self) {
        for held in self.lines.held_lines() {
            if let Some(event) = event_if_new(&held, &mut self.sent) {
                self.hub.publish(&event);
            }
        }
    }
}

#[cfg(test)]
#[path = "agent_line_bridge.tests.rs"]
mod tests;
