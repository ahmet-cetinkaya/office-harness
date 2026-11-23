// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, PoisonError};

use agent_application::commands::change_agent_status::ChangeAgentStatusHandler;
use agent_domain::enums::StatusEvent;
use agent_presentation::agent_status_event;
use kernel::AgentId;
use terminal_application::common::interfaces::TerminalRuntime;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use super::agent_status_bridge::StatusOrder;
use super::hook_sessions::HookSessions;
use super::notification_bridge::NotificationBridge;
use super::wait_tracker::WaitTracker;
use crate::bootstrap::StorageAlerts;
use crate::server::{EventHub, HookReport, HookReportSink};

/// One thing that moves the status of an exact session (0024 D7).
enum HookWork {
    Report(HookReport),
    /// A person's input ended the agent's permission ask.
    InputEndsAsk(AgentId),
}

/// Takes the reports that passed the endpoint and queues them. One consumer
/// applies them one at a time in the order they arrived, because the CLI
/// reports its next event as soon as the server has answered the last one.
struct HookWorkQueue(UnboundedSender<HookWork>);

impl HookReportSink for HookWorkQueue {
    fn accept(&self, report: HookReport) {
        // A closed queue means that the server is stopping.
        let _ = self.0.send(HookWork::Report(report));
    }
}

/// Turns the reports of `claude` sessions into status changes with the same
/// status machine and the same `agent.status` frames as the terminal
/// events (0024 D7), and tells every client.
#[derive(Clone)]
pub struct HookStatusBridge {
    pub change_status: ChangeAgentStatusHandler,
    pub runtime: Arc<dyn TerminalRuntime>,
    pub hub: EventHub,
    pub order: StatusOrder,
    pub alerts: StorageAlerts,
    pub hooks: HookSessions,
    /// Records an ask that no status frame carries (0024 D7).
    pub notifications: NotificationBridge,
}

impl HookStatusBridge {
    /// Starts the consumer and returns the sink that the endpoint delivers
    /// to. `waits` tells the bridge when a person's input ends a permission
    /// ask.
    #[must_use]
    pub fn spawn(self, waits: &WaitTracker) -> Arc<dyn HookReportSink> {
        let (sender, mut queue) = unbounded_channel();
        let input_sender = sender.clone();
        waits.on_ask_wait_end(move |agent_id| {
            let _ = input_sender.send(HookWork::InputEndsAsk(agent_id));
        });
        tokio::spawn(async move {
            while let Some(work) = queue.recv().await {
                let bridge = self.clone();
                // The store write blocks, so it leaves the async threads.
                let _ = tokio::task::spawn_blocking(move || bridge.apply(work)).await;
            }
        });
        Arc::new(HookWorkQueue(sender))
    }

    fn apply(&self, work: HookWork) {
        // Starts hold the order, so the run cannot change while the work is
        // judged and applied.
        let _turn = self.order.lock().unwrap_or_else(PoisonError::into_inner);
        match work {
            HookWork::Report(report) => self.apply_report(&report),
            HookWork::InputEndsAsk(agent_id) => {
                if let Some(event) = self.hooks.input_ends_ask(agent_id) {
                    let _ = self.change(agent_id, event, None);
                }
            }
        }
    }

    fn apply_report(&self, report: &HookReport) {
        let Some(run) = self.runtime.current_run(report.agent_id) else {
            return;
        };
        let Some(outcome) = self.hooks.apply_report(report, run) else {
            return;
        };
        let changed = self.change(report.agent_id, outcome.status_event, outcome.detail);
        // The status already showed `waiting for input`, so no frame will
        // tell the notification bridge of the ask.
        if outcome.ask_opened && !changed {
            self.notifications.record_permission_ask(report.agent_id);
        }
    }

    /// Applies the event and tells the clients. True when the status changed.
    fn change(&self, agent_id: AgentId, event: StatusEvent, detail: Option<&str>) -> bool {
        match self.change_status.handle(agent_id, event) {
            Ok(Some(agent)) => {
                self.hub
                    .publish(&agent_status_event(agent.id, agent.status, detail));
                true
            }
            Ok(None) => false,
            Err(error) => {
                self.alerts.report_disk_failure(&error.to_string());
                false
            }
        }
    }
}
