// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::{AgentStore, Clock};
use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use kernel::{AgentId, Timestamp};
use notification_application::commands::create_notification::{
    CreateNotification, CreateNotificationHandler,
};
use notification_application::commands::needs_you::NeedsYouClearReason;
use notification_application::common::errors::NotificationApplicationError;
use notification_domain::{
    AttentionEvent, AttentionKind, ItemReport, Notification, NotificationEvent, NotificationId,
};
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_domain::events::TerminalEvent;
use tokio::sync::broadcast::error::RecvError;
use wire::Event;

use super::NeedsYouBridge;
use super::hook_sessions::{DETAIL_PERMISSION_ASKED, DETAIL_TURN_FAILED, DETAIL_TURN_STOPPED};
use super::terminal_events::AgentTerminalEvent;
use super::wait_tracker::{Change, StatusChange, WaitTracker};
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
    /// Judges each change between `working` and `waiting for input`; this
    /// bridge is the only one that asks, so each change is judged once.
    waits: WaitTracker,
    /// The agents whose last status frame was `waiting_for_input`: the next
    /// `working` of such an agent is a change out of a wait, and a `working`
    /// that follows `starting` is not (0022 D5).
    waiting_agents: Arc<Mutex<HashSet<AgentId>>>,
    /// The missed-runs notification of the minute that last had one, which a
    /// later round in the same minute extends (0025/FR-006).
    missed_notice: Arc<Mutex<Option<MissedNotice>>>,
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
        waits: WaitTracker,
        hub: EventHub,
        alerts: StorageAlerts,
        log: ServerLog,
    ) -> Self {
        clear_on_input_while_working(&waits, Arc::clone(&agents), needs_you.clone());
        Self {
            create,
            agents,
            runtime,
            clock,
            quiet_period_ms,
            needs_you,
            waits,
            waiting_agents: Arc::default(),
            missed_notice: Arc::default(),
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
        if let Some((change, Change::Unprompted)) = self.judge(&status) {
            // A redraw of the screen: it clears nothing and records nothing
            // (0022 FR-001), and a `working` one is bounded by a timer.
            if change == StatusChange::ToWorking {
                self.arm_work_bound(status.agent_id);
            }
            return;
        }
        if let Some(reason) = status.clear_reason() {
            self.needs_you.clear(status.agent_id, reason);
        }
        if let Some(event) = status.notification_event() {
            let attention_detail = self.attention_detail(event, status.detail);
            self.record(status.agent_id, event, None, attention_detail);
        }
    }

    /// Asks the tracker about the frame once, when the frame is a change
    /// between `working` and `waiting for input`.
    fn judge(&self, status: &StatusFrame) -> Option<(StatusChange, Change)> {
        let change = self.status_change(status)?;
        Some((change, self.waits.classify(status.agent_id, change)))
    }

    /// The change between the two statuses that the frame makes, and the
    /// bookkeeping of which agents are waiting.
    fn status_change(&self, status: &StatusFrame) -> Option<StatusChange> {
        let mut waiting = self
            .waiting_agents
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if status.name == "waiting_for_input" {
            waiting.insert(status.agent_id);
            return Some(StatusChange::ToWaitingForInput);
        }
        let was_waiting = waiting.remove(&status.agent_id);
        (status.name == "working" && was_waiting).then_some(StatusChange::ToWorking)
    }

    /// Removes the agent's item when its unprompted stretch of `working`
    /// passes the work bound: long work is not a redraw, so the item no
    /// longer describes the agent (0022 D4). One timer per stretch; a timer
    /// whose stretch is over finds a different start and does nothing.
    fn arm_work_bound(&self, agent_id: AgentId) {
        let Some(since) = self.waits.working_since(agent_id) else {
            return;
        };
        let deadline = tokio::time::Instant::from_std(since + self.waits.work_bound());
        let bridge = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep_until(deadline).await;
            let _ = tokio::task::spawn_blocking(move || {
                bridge.end_stretch_at_bound(agent_id, since);
            })
            .await;
        });
    }

    fn end_stretch_at_bound(&self, agent_id: AgentId, since: std::time::Instant) {
        if self.waits.working_since(agent_id) != Some(since)
            || !shows_working(self.agents.as_ref(), agent_id)
        {
            return;
        }
        // The return reads as prompted even when this timer ran late, and
        // the wait end clears the item (see `clear_on_input_while_working`).
        self.waits.wait_end(agent_id);
    }

    /// What the queue shows beside the event (0008 D3): the terminal's quiet
    /// period for a wait, the status frame's text for an end.
    fn attention_detail(
        &self,
        event: NotificationEvent,
        frame_detail: Option<String>,
    ) -> Option<String> {
        match event {
            NotificationEvent::WaitingForInput => Some(
                frame_detail
                    .filter(|word| word == DETAIL_TURN_STOPPED || word == DETAIL_TURN_FAILED)
                    .unwrap_or_else(|| self.quiet_period_ms.to_string()),
            ),
            NotificationEvent::PermissionAsked => None,
            _ => frame_detail,
        }
    }

    /// Records that an exact session asked for permission while its status
    /// already showed `waiting for input`, so that no status frame carries
    /// the event (0024/FR-013).
    pub fn record_permission_ask(&self, agent_id: AgentId) {
        self.record(agent_id, NotificationEvent::PermissionAsked, None, None);
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

    /// Records that a temp ended for the idle limit and no parent heard of it
    /// (0020/FR-007). The temp is gone or going, so the caller gives its name,
    /// and no Needs you item follows: nothing would ever clear it.
    pub fn record_temp_ended(&self, agent_id: AgentId, name: &str, reason: &str) {
        let created = self.create.handle(CreateNotification {
            agent_id: Some(agent_id),
            agent_name: name.to_owned(),
            event: NotificationEvent::TempEnded,
            at: self.clock.now(),
            detail: Some(reason.to_owned()),
        });
        match created {
            Ok(Some(notification)) => self.hub.publish(&created_event(&notification)),
            Ok(None) => {}
            Err(error) => self.report(&error, agent_id, NotificationEvent::TempEnded),
        }
    }

    /// Records that a run of a schedule did not deliver, or that the office
    /// switched the schedule off (0025/FR-011): the schedule's name and the
    /// target's id, and the text of the result as the detail. `target` is
    /// `None` when the target is deleted, so the row names no agent that is
    /// gone. It adds no Needs you item, because a schedule is no agent (0025
    /// A6).
    pub fn record_schedule_result(
        &self,
        schedule_name: &str,
        target: Option<AgentId>,
        detail: String,
    ) {
        let created = self.create.handle(CreateNotification {
            agent_id: target,
            agent_name: schedule_name.to_owned(),
            event: NotificationEvent::Schedule,
            at: self.clock.now(),
            detail: Some(detail),
        });
        match created {
            Ok(Some(notification)) => self.hub.publish(&created_event(&notification)),
            Ok(None) => {}
            Err(error) => match target {
                Some(agent_id) => self.report(&error, agent_id, NotificationEvent::Schedule),
                None => self.report_schedule_row(&error),
            },
        }
    }

    /// Records that `schedules` schedules missed runs at `at`. One
    /// notification serves a whole minute: a call in a minute that already
    /// has one adds its count to that row (0025/FR-006, D5).
    pub fn record_missed_schedules(&self, schedules: u32, at: Timestamp) {
        let minute = at.as_unix_millis().div_euclid(MILLIS_PER_MINUTE);
        let mut notice = self
            .missed_notice
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(open) = notice.as_mut().filter(|open| open.minute == minute) {
            open.schedules += schedules;
            open.notification.agent_name = missed_text(open.schedules);
            match self.create.amend(&open.notification) {
                Ok(amended) => self.hub.publish(&created_event(&amended)),
                Err(error) => self.report_schedule_row(&error),
            }
            return;
        }
        let created = self.create.handle(CreateNotification {
            agent_id: None,
            agent_name: missed_text(schedules),
            event: NotificationEvent::Schedule,
            at,
            detail: None,
        });
        match created {
            Ok(Some(notification)) => {
                self.hub.publish(&created_event(&notification));
                *notice = Some(MissedNotice {
                    minute,
                    schedules,
                    notification,
                });
            }
            Ok(None) => {}
            Err(error) => self.report_schedule_row(&error),
        }
    }

    fn report_schedule_row(&self, error: &NotificationApplicationError) {
        let message = format!("a schedule notification could not be stored: {error}");
        if error.is_disk_failure() {
            self.alerts.report_disk_failure(&message);
        } else {
            self.log.error(&message);
        }
    }

    /// Records that the sweep after an unclean stop could not end a process
    /// of the agent; the reason travels as the row's detail (0017/FR-006).
    pub fn record_leftover_kept(&self, agent_id: AgentId, reason: &str) {
        let detail = Some(reason.to_owned());
        self.record(
            agent_id,
            NotificationEvent::LeftoverKept,
            detail.clone(),
            detail,
        );
    }

    /// Records a report for the person (0015/FR-007): a `report`
    /// notification naming the kind, coalesced while one is unread, and the
    /// report on the agent's item. `text` is already in visible form.
    pub fn record_report(&self, agent_id: AgentId, kind: &str, text: String) {
        self.notify_then_queue(
            agent_id,
            NotificationEvent::Report,
            Some(kind.to_owned()),
            |at, origin, announce_created| {
                let report = ItemReport {
                    kind: kind.to_owned(),
                    text,
                    at,
                };
                self.needs_you
                    .record_report(agent_id, report, origin, announce_created);
            },
        );
    }

    fn record(
        &self,
        agent_id: AgentId,
        event: NotificationEvent,
        notification_detail: Option<String>,
        attention_detail: Option<String>,
    ) {
        self.notify_then_queue(
            agent_id,
            event,
            notification_detail,
            |at, origin, announce_created| {
                let Some(kind) = attention_kind_of(event) else {
                    return;
                };
                let attention = AttentionEvent {
                    kind,
                    detail: attention_detail,
                    at,
                };
                self.needs_you
                    .record(agent_id, attention, origin, announce_created);
            },
        );
    }

    /// The notification first, then the queue: the queue item names the
    /// notification that created it, and the clients hear
    /// `notification.created` after the item is stored and before
    /// `needs_you.changed`. The queue is fed even when no notification was
    /// created, so a coalesced bell or report still updates its item. An
    /// agent that is already deleted keeps its notification but gets no
    /// item: the delete cleanup has run, so nothing would ever clear it.
    fn notify_then_queue(
        &self,
        agent_id: AgentId,
        event: NotificationEvent,
        notification_detail: Option<String>,
        queue: impl FnOnce(Timestamp, Option<NotificationId>, &dyn Fn()),
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
        let origin = created.as_ref().map(|notification| notification.id);
        queue(at, origin, &announce_created);
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
            agent_id: Some(agent_id),
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
const MILLIS_PER_MINUTE: i64 = 60_000;

/// The missed-runs notification of one minute, with the number of schedules
/// that it counts.
struct MissedNotice {
    minute: i64,
    schedules: u32,
    notification: Notification,
}

fn missed_text(schedules: u32) -> String {
    match schedules {
        1 => "1 schedule missed runs".to_owned(),
        count => format!("{count} schedules missed runs"),
    }
}

/// What one published `agent.status` frame says.
struct StatusFrame {
    agent_id: AgentId,
    name: String,
    detail: Option<String>,
}

impl StatusFrame {
    /// Why the status clears the agent's queue item, if it does (0008 D2):
    /// the agent moved on, or was stopped. `interrupted` clears nothing: a
    /// server restart marks agents interrupted, and the item must survive it.
    /// A resume leaves an item that holds a report (0015/FR-008).
    fn clear_reason(&self) -> Option<NeedsYouClearReason> {
        match self.name.as_str() {
            "starting" | "working" => Some(NeedsYouClearReason::Resumed),
            "stopped" => Some(NeedsYouClearReason::Stopped),
            _ => None,
        }
    }

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
            "waiting_for_input" if self.detail.as_deref() == Some(DETAIL_PERMISSION_ASKED) => {
                Some(NotificationEvent::PermissionAsked)
            }
            "waiting_for_input" => Some(NotificationEvent::WaitingForInput),
            _ => None,
        }
    }
}

/// The queue kind of an event, or `None` for an event that never reaches the
/// Needs you queue.
const fn attention_kind_of(event: NotificationEvent) -> Option<AttentionKind> {
    Some(match event {
        NotificationEvent::Exited => AttentionKind::Exited,
        NotificationEvent::Failed => AttentionKind::Failed,
        NotificationEvent::WaitingForInput => AttentionKind::WaitingForInput,
        NotificationEvent::Bell => AttentionKind::Bell,
        NotificationEvent::AutoHold => AttentionKind::AutoHold,
        NotificationEvent::ResumeHold => AttentionKind::ResumeHold,
        NotificationEvent::LeftoverKept => AttentionKind::LeftoverKept,
        NotificationEvent::Report => AttentionKind::Report,
        NotificationEvent::PermissionAsked => AttentionKind::PermissionAsked,
        NotificationEvent::TempEnded | NotificationEvent::Schedule => return None,
    })
}

fn created_event(notification: &Notification) -> Event {
    notification_presentation::notification_created_event(notification)
}

/// A person's input or office text that reaches an agent while it shows
/// `working` removes its item, as a resume does (0008/FR-005 as modified by
/// 0022): the person has dealt with the agent, whatever the screen redraws.
fn clear_on_input_while_working(
    waits: &WaitTracker,
    agents: Arc<dyn AgentStore>,
    needs_you: NeedsYouBridge,
) {
    waits.on_wait_end(move |agent_id| {
        if shows_working(agents.as_ref(), agent_id) {
            needs_you.clear(agent_id, NeedsYouClearReason::Resumed);
        }
    });
}

fn shows_working(agents: &dyn AgentStore, agent_id: AgentId) -> bool {
    matches!(
        agents.find(agent_id),
        Ok(Some(agent)) if agent.status == AgentStatus::Working
    )
}

#[cfg(test)]
#[path = "notification_bridge.tests.rs"]
mod tests;
