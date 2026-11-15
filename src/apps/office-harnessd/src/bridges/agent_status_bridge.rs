// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use agent_application::commands::change_agent_status::ChangeAgentStatusHandler;
use agent_domain::enums::StatusEvent;
use agent_presentation::agent_status_event;
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_domain::events::TerminalEvent;
use tokio::sync::broadcast::{self, error::RecvError};

use super::terminal_events::AgentTerminalEvent;
use crate::bootstrap::StorageAlerts;
use crate::server::EventHub;

/// Held while `agent.start` runs and publishes its `agent.status` event, and
/// while a terminal event changes a status and publishes it, so that
/// `starting` is never announced after the `working` that follows it.
pub type StatusOrder = Arc<Mutex<()>>;

/// Turns the terminal module's events into agent status changes (FR-014) and
/// tells every client. An event of an earlier start of the terminal is
/// dropped, so a late exit never ends the run that replaced it.
#[derive(Clone)]
pub struct AgentStatusBridge {
    pub change_status: ChangeAgentStatusHandler,
    pub runtime: Arc<dyn TerminalRuntime>,
    pub hub: EventHub,
    pub order: StatusOrder,
    pub alerts: StorageAlerts,
}

impl AgentStatusBridge {
    /// Runs until the event source is dropped.
    pub fn spawn(self, mut events: broadcast::Receiver<AgentTerminalEvent>) {
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(item) => {
                        let bridge = self.clone();
                        // The store write blocks, so it leaves the async threads.
                        let _ = tokio::task::spawn_blocking(move || bridge.apply(item)).await;
                    }
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }

    fn apply(&self, item: AgentTerminalEvent) {
        // Starts hold the order, so the current run cannot change while this
        // event is judged and applied.
        let _turn = self.order.lock().unwrap_or_else(PoisonError::into_inner);
        if self.runtime.current_run(item.agent_id) != Some(item.run) {
            return;
        }
        if item.event == TerminalEvent::OutputStorageFailed {
            self.alerts.report_disk_failure(&format!(
                "the output file of agent {} cannot be written",
                item.agent_id
            ));
            return;
        }
        let Some((event, detail)) = status_event(item.event) else {
            return;
        };
        match self.change_status.handle(item.agent_id, event) {
            Ok(Some(agent)) => self.hub.publish(&agent_status_event(
                agent.id,
                agent.status,
                detail.as_deref(),
            )),
            Ok(None) => {}
            Err(error) => self.alerts.report_disk_failure(&error.to_string()),
        }
    }
}

/// The status event for a terminal event, and the detail that a failure shows.
fn status_event(event: TerminalEvent) -> Option<(StatusEvent, Option<String>)> {
    match event {
        TerminalEvent::FirstOutput => Some((StatusEvent::FirstOutput, None)),
        TerminalEvent::OutputResumed => Some((StatusEvent::Output, None)),
        TerminalEvent::QuietPeriod => Some((StatusEvent::QuietPeriodElapsed, None)),
        TerminalEvent::Exited { exit_code: 0 } => Some((StatusEvent::ExitedWithCodeZero, None)),
        TerminalEvent::Exited { exit_code } => Some((
            StatusEvent::ExitedWithFailure,
            Some(format!("the command ended with exit code {exit_code}")),
        )),
        // The child keeps running; the bridge raises the alert instead.
        TerminalEvent::OutputStorageFailed => None,
    }
}

#[cfg(test)]
#[path = "agent_status_bridge.tests.rs"]
mod tests;
