// SPDX-License-Identifier: AGPL-3.0-or-later

use std::str::FromStr;
use std::sync::Arc;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::{AgentStore, Clock};
use agent_domain::entities::Agent;
use kernel::{AgentId, Timestamp};
use notification_application::commands::create_notification::{
    CreateNotification, CreateNotificationHandler,
};
use notification_application::commands::needs_you::NeedsYouClearReason;
use notification_application::common::errors::NotificationApplicationError;
use notification_domain::{AttentionEvent, AttentionKind, Notification, NotificationEvent};
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_domain::events::TerminalEvent;
use tokio::sync::broadcast::error::RecvError;
use wire::Event;

use super::NeedsYouBridge;
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
///
/// Every one of these events also goes to the Needs you queue, even a bell
/// that the notification rules coalesce (0008 D3). The queue is cleared by the
/// statuses that show the agent moved on (0008 D2).
#[derive(Clone)]
pub struct NotificationBridge {
    create: CreateNotificationHandler,
    agents: Arc<dyn AgentStore>,
    runtime: Arc<dyn TerminalRuntime>,
    clock: Arc<dyn Clock>,
    /// The quiet period the terminal runtime was started with, the value the
    /// terminal waits for before it calls an agent waiting.
    quiet_period_ms: u64,
    needs_you: NeedsYouBridge,
    hub: EventHub,
    alerts: StorageAlerts,
    log: ServerLog,
}

impl NotificationBridge {
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "the bridge joins the notification, queue, agent, terminal, and settings services"
    )]
    pub fn new(
        create: CreateNotificationHandler,
        agents: Arc<dyn AgentStore>,
        runtime: Arc<dyn TerminalRuntime>,
        clock: Arc<dyn Clock>,
        quiet_period_ms: u64,
        needs_you: NeedsYouBridge,
        hub: EventHub,
        alerts: StorageAlerts,
        log: ServerLog,
    ) -> Self {
        Self {
            create,
            agents,
            runtime,
            clock,
            quiet_period_ms,
            needs_you,
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
    /// can resync with `notification.list` and `needs_you.list`.
    fn missed(&self, count: u64, source: &str) {
        self.alerts.report_disk_failure(&format!(
            "the notification bridge missed {count} {source}; \
             notifications are missing for them; \
             read notification.list and needs_you.list again"
        ));
    }

    /// Records what a published `agent.status` frame asks for.
    fn apply_frame(&self, frame: &str) {
        let Some(status) = StatusFrame::parse(frame) else {
            return;
        };
        if CLEARING_STATUSES.contains(&status.name.as_str()) {
            self.needs_you
                .clear(status.agent_id, NeedsYouClearReason::StatusChanged);
        }
        if let Some(event) = status.notification_event() {
            let attention_detail = self.attention_detail(event, status.detail);
            self.record(status.agent_id, event, None, attention_detail);
        }
    }

    /// What the queue shows beside the event (0008 D3): the terminal's quiet
    /// period for a wait, the status frame's text for an end.
    fn attention_detail(
        &self,
        event: NotificationEvent,
        frame_detail: Option<String>,
    ) -> Option<String> {
        if event == NotificationEvent::WaitingForInput {
            return Some(self.quiet_period_ms.to_string());
        }
        frame_detail
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
        self.record(item.agent_id, NotificationEvent::Bell, None, None);
    }

    /// Records that the office held the agent after its limit refusals; the
    /// limit's name travels as the row's detail (0007 D7).
    pub fn record_auto_hold(&self, agent_id: AgentId, limit_name: &str) {
        let detail = Some(limit_name.to_owned());
        self.record(
            agent_id,
            NotificationEvent::AutoHold,
            detail.clone(),
            detail,
        );
    }

    /// Records that the office held the agent after its resume failed twice;
    /// the reason travels as the row's detail (0013).
    pub fn record_resume_hold(&self, agent_id: AgentId, reason: &str) {
        let detail = Some(reason.to_owned());
        self.record(
            agent_id,
            NotificationEvent::ResumeHold,
            detail.clone(),
            detail,
        );
    }

    /// The notification first, then the queue: the queue item names the
    /// notification that created it, and the clients hear
    /// `notification.created` after the item is stored and before
    /// `needs_you.changed`. The queue is fed even when no notification was
    /// created, so a coalesced bell still updates its item. An agent that is
    /// already deleted keeps its notification but gets no item: the delete
    /// cleanup has run, so nothing would ever clear it.
    fn record(
        &self,
        agent_id: AgentId,
        event: NotificationEvent,
        notification_detail: Option<String>,
        attention_detail: Option<String>,
    ) {
        let at = self.clock.now();
        let agent = self.agents.find(agent_id);
        let created = self.create_notification(agent_id, &agent, event, at, notification_detail);
        let announce_created = || {
            if let Some(notification) = &created {
                self.hub.publish(&created_event(notification));
            }
        };
        if matches!(agent, Ok(None)) {
            announce_created();
            return;
        }
        let attention = AttentionEvent {
            kind: attention_kind_of(event),
            detail: attention_detail,
            at,
        };
        let origin = created.as_ref().map(|notification| notification.id);
        self.needs_you
            .record(agent_id, attention, origin, announce_created);
    }

    /// Creates the notification, copying the agent's name now; the row is
    /// published by the caller. A deleted agent has no name, so the row says
    /// so (spec edge case). A store failure is not a delete: it stops the
    /// record, so nothing claims a name nobody confirmed.
    fn create_notification(
        &self,
        agent_id: AgentId,
        agent: &Result<Option<Agent>, PortError>,
        event: NotificationEvent,
        at: Timestamp,
        detail: Option<String>,
    ) -> Option<Notification> {
        let name = match agent {
            Ok(Some(agent)) => agent.name.as_str().to_owned(),
            Ok(None) => DELETED_AGENT_NAME.to_owned(),
            Err(error) => {
                self.report_lookup_failure(error, agent_id, event);
                return None;
            }
        };
        let created = self.create.handle(CreateNotification {
            agent_id,
            agent_name: name,
            event,
            at,
            detail,
        });
        // A coalesced bell adds no row, so `Ok(None)` makes no notification.
        created.unwrap_or_else(|error| {
            self.report(&error, agent_id, event);
            None
        })
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

/// The statuses that show the agent moved on, so its queue item goes
/// (0008 D2). `interrupted` is absent: a server restart marks agents
/// interrupted, and the item must survive it.
const CLEARING_STATUSES: [&str; 3] = ["starting", "working", "stopped"];

/// What one published `agent.status` frame says.
struct StatusFrame {
    agent_id: AgentId,
    name: String,
    detail: Option<String>,
}

impl StatusFrame {
    /// `None` for anything that is not an `agent.status` frame of an agent.
    fn parse(frame: &str) -> Option<Self> {
        let body: serde_json::Value = serde_json::from_str(frame).ok()?;
        if body.get("type").and_then(serde_json::Value::as_str) != Some("agent.status") {
            return None;
        }
        let text_of = |key: &str| body.get(key)?.as_str().map(str::to_owned);
        Some(Self {
            agent_id: AgentId::from_str(&text_of("agent_id")?).ok()?,
            name: text_of("status")?,
            detail: text_of("detail"),
        })
    }

    /// The event this status asks to record, if any. `stopped` and
    /// `interrupted` are the user's own action, so they record nothing (US2
    /// scenario 7, FR-006; 0008 PB-3).
    fn notification_event(&self) -> Option<NotificationEvent> {
        match self.name.as_str() {
            "exited" => Some(NotificationEvent::Exited),
            "failed" => Some(NotificationEvent::Failed),
            "waiting_for_input" => Some(NotificationEvent::WaitingForInput),
            _ => None,
        }
    }
}

const fn attention_kind_of(event: NotificationEvent) -> AttentionKind {
    match event {
        NotificationEvent::Exited => AttentionKind::Exited,
        NotificationEvent::Failed => AttentionKind::Failed,
        NotificationEvent::WaitingForInput => AttentionKind::WaitingForInput,
        NotificationEvent::Bell => AttentionKind::Bell,
        NotificationEvent::AutoHold => AttentionKind::AutoHold,
        NotificationEvent::ResumeHold => AttentionKind::ResumeHold,
    }
}

fn created_event(notification: &Notification) -> Event {
    notification_presentation::notification_created_event(notification)
}

#[cfg(test)]
#[path = "notification_bridge.tests.rs"]
mod tests;
