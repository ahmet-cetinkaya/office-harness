// SPDX-License-Identifier: AGPL-3.0-or-later

use std::str::FromStr;
use std::sync::Arc;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::{AgentStore, Clock};
use kernel::AgentId;
use notification_application::commands::create_notification::{
    CreateNotification, CreateNotificationHandler,
};
use notification_application::common::errors::NotificationApplicationError;
use notification_domain::{Notification, NotificationEvent};
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_domain::events::TerminalEvent;
use tokio::sync::broadcast::error::RecvError;
use wire::Event;

use super::terminal_events::AgentTerminalEvent;
use crate::bootstrap::{ServerLog, StorageAlerts};
use crate::server::EventHub;

/// Turns the office's agent status events and the terminals' bells into
/// notification rows (FR-006, 0003 T3b). The status frames are the ones the
/// status bridge and the gateways already published, so a notification
/// follows only a real status-machine transition: `working` to
/// `waiting for input` only from `working`, and nothing for `stopped` or
/// `interrupted`. The agent name is copied at creation, so the row outlives a
/// deleted agent. A failed lookup or row write records nothing and is
/// reported like any storage failure, and a lagged broadcast is surfaced as
/// one too, so the client learns which notifications are missing.
#[derive(Clone)]
pub struct NotificationBridge {
    create: CreateNotificationHandler,
    agents: Arc<dyn AgentStore>,
    runtime: Arc<dyn TerminalRuntime>,
    clock: Arc<dyn Clock>,
    hub: EventHub,
    alerts: StorageAlerts,
    log: ServerLog,
}

impl NotificationBridge {
    #[must_use]
    pub fn new(
        create: CreateNotificationHandler,
        agents: Arc<dyn AgentStore>,
        runtime: Arc<dyn TerminalRuntime>,
        clock: Arc<dyn Clock>,
        hub: EventHub,
        alerts: StorageAlerts,
        log: ServerLog,
    ) -> Self {
        Self {
            create,
            agents,
            runtime,
            clock,
            hub,
            alerts,
            log,
        }
    }

    /// Runs until both sources are dropped.
    pub fn spawn(
        self,
        mut statuses: tokio::sync::broadcast::Receiver<Arc<str>>,
        mut bells: tokio::sync::broadcast::Receiver<AgentTerminalEvent>,
    ) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    frame = statuses.recv() => match frame {
                        Ok(text) => {
                            // The store write blocks, so it leaves the async
                            // threads.
                            let bridge = self.clone();
                            let _ =
                                tokio::task::spawn_blocking(move || bridge.apply_frame(&text))
                                    .await;
                        }
                        Err(RecvError::Closed) => return,
                        Err(RecvError::Lagged(missed)) => self.missed(missed, "status events"),
                    },
                    item = bells.recv() => match item {
                        Ok(event) => {
                            let bridge = self.clone();
                            let _ = tokio::task::spawn_blocking(move || bridge.apply_bell(event))
                                .await;
                        }
                        Err(RecvError::Closed) => return,
                        Err(RecvError::Lagged(missed)) => self.missed(missed, "terminal events"),
                    },
                }
            }
        });
    }

    /// Reports the events a full broadcast channel dropped: FR-006
    /// notifications are missing for them, so the client learns of the loss
    /// as a storage alert, the same channel a failed row write takes, and
    /// can resync with `notification.list`.
    fn missed(&self, count: u64, source: &str) {
        self.alerts.report_disk_failure(&format!(
            "the notification bridge missed {count} {source}; \
             notifications are missing for them"
        ));
    }

    /// Records what a published `agent.status` frame asks for.
    fn apply_frame(&self, frame: &str) {
        let Some((agent_id, event)) = notification_of_frame(frame) else {
            return;
        };
        self.record(agent_id, event);
    }

    /// Records a bell of the terminal's current run; an earlier run's bell is
    /// dropped, like the status bridge drops its late exits.
    fn apply_bell(&self, item: AgentTerminalEvent) {
        if item.event != TerminalEvent::Bell {
            return;
        }
        if self.runtime.current_run(item.agent_id) != Some(item.run) {
            return;
        }
        self.record(item.agent_id, NotificationEvent::Bell);
    }

    fn record(&self, agent_id: AgentId, event: NotificationEvent) {
        let name = match self.name_of(agent_id) {
            Ok(name) => name,
            Err(error) => return self.report_lookup_failure(&error, agent_id, event),
        };
        let created = self.create.handle(CreateNotification {
            agent_id,
            agent_name: name,
            event,
            at: self.clock.now(),
        });
        match created {
            // A coalesced bell adds no row and no event.
            Ok(Some(notification)) => self.hub.publish(&created_event(&notification)),
            Ok(None) => {}
            Err(error) => self.report(&error, agent_id, event),
        }
    }

    /// The agent's name now, copied into the row. A deleted agent has none,
    /// so the row says so (spec edge case). A store failure is not a delete:
    /// it stops the record, so nothing claims a name nobody confirmed.
    fn name_of(&self, agent_id: AgentId) -> Result<String, PortError> {
        match self.agents.find(agent_id)? {
            Some(agent) => Ok(agent.name.as_str().to_owned()),
            None => Ok(DELETED_AGENT_NAME.to_owned()),
        }
    }

    fn report(
        &self,
        error: &NotificationApplicationError,
        agent_id: AgentId,
        event: NotificationEvent,
    ) {
        let message =
            format!("the {event} notification of agent {agent_id} could not be stored: {error}");
        if error.is_disk_failure() {
            self.alerts.report_disk_failure(&message);
        } else {
            self.log.error(&message);
        }
    }

    /// Reports a failed name lookup like a failed row write: no row is
    /// recorded, so the notification is lost, and the failure is told the
    /// same way.
    fn report_lookup_failure(
        &self,
        error: &PortError,
        agent_id: AgentId,
        event: NotificationEvent,
    ) {
        let message =
            format!("the {event} notification of agent {agent_id} could not be stored: {error}");
        if matches!(
            error,
            PortError::Storage {
                is_disk_failure: true,
                ..
            }
        ) {
            self.alerts.report_disk_failure(&message);
        } else {
            self.log.error(&message);
        }
    }
}

const DELETED_AGENT_NAME: &str = "deleted agent";

/// The (agent, event) that one published status frame asks to record, if any.
/// `stopped` and `interrupted` are the user's own action, so they record
/// nothing (US2 scenario 7, FR-006).
fn notification_of_frame(frame: &str) -> Option<(AgentId, NotificationEvent)> {
    let body: serde_json::Value = serde_json::from_str(frame).ok()?;
    if body.get("type").and_then(serde_json::Value::as_str) != Some("agent.status") {
        return None;
    }
    let agent_id = AgentId::from_str(body.get("agent_id")?.as_str()?).ok()?;
    let event = match body.get("status").and_then(serde_json::Value::as_str)? {
        "exited" => NotificationEvent::Exited,
        "failed" => NotificationEvent::Failed,
        "waiting_for_input" => NotificationEvent::WaitingForInput,
        _ => return None,
    };
    Some((agent_id, event))
}

fn created_event(notification: &Notification) -> Event {
    notification_presentation::notification_created_event(notification)
}

#[cfg(test)]
#[path = "notification_bridge.tests.rs"]
mod tests;
