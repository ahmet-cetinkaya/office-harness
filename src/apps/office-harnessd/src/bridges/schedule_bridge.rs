// SPDX-License-Identifier: AGPL-3.0-or-later

//! Joins the schedule module to the agents, the holds, the queue, the wake,
//! the action record, and the notifications (0025 D9 to D14). The scheduler
//! task starts runs through it; `schedule_run_job.rs` is the run itself.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use agent_application::common::action_journal::ActionJournal;
use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::AgentStore;
use agent_application::common::models::AgentAction;
use hierarchy_application::common::ports::{ScheduledOutcome, ScheduledOutcomes};
use hierarchy_application::{
    HoldService, TreeQuery, commands::send_directive::SendDirectiveHandler,
};
use jiff::tz::TimeZone;
use kernel::{Actor, AgentId, ScheduleId, Timestamp};
use schedule_application::common::errors::ScheduleApplicationError;
use schedule_application::common::interfaces::{
    RunNow, ScheduleCapSource, ScheduleStore, TargetCheck,
};
use schedule_application::common::models::{NewRun, RunId, RunOutcome, RunResult, RunVia};
use schedule_application::services::{
    DueRun, MissedBatch, RunBook, SETTLED_AT_START_REASON, WAITING_REASON,
};
use schedule_domain::{OffReason, Schedule};
use schedule_presentation::schedule_changed_event;
use tokio::runtime::Handle;
use tokio::sync::watch;

use super::{
    NotificationBridge, ScheduleLabels, ScheduleTime, SleepingAgents, take_recorded_events,
};
use crate::bootstrap::{SERVER_STOPPED, ServerLog, StorageAlerts};
use crate::server::{EventHub, ResumeContext};

const TARGET_REMOVED_NOTICE: &str = "its target was deleted or removed, so the schedule is off";

/// What the bridge reads and writes. Every field is shared state or a handle.
#[derive(Clone)]
pub struct ScheduleParts {
    pub store: Arc<dyn ScheduleStore>,
    pub agents: Arc<dyn AgentStore>,
    pub tree: Arc<TreeQuery>,
    pub holds: Arc<HoldService>,
    pub directives: Arc<SendDirectiveHandler>,
    pub resume: ResumeContext,
    pub sleeping: SleepingAgents,
    pub journal: ActionJournal,
    pub notifications: NotificationBridge,
    pub events: EventHub,
    pub alerts: StorageAlerts,
    pub labels: ScheduleLabels,
    pub cap: Arc<dyn ScheduleCapSource>,
    pub time: &'static dyn ScheduleTime,
    pub stopping: watch::Receiver<bool>,
    pub runtime: Handle,
    pub log: ServerLog,
}

#[derive(Default)]
struct BridgeState {
    /// Held by whoever reads a schedule and writes it back, so a save of the
    /// person, a round, a removed target, and a settled run never overwrite
    /// each other's change.
    schedules: Mutex<()>,
    /// Held while a run checks the previous one and opens its own row, so
    /// two runs of one schedule never both pass the check (FR-008).
    starts: Mutex<()>,
    /// The runs whose directive waits in a queue, by run id, with the
    /// schedule and the target: a run settles also after its schedule is
    /// deleted, and its entries still name the schedule.
    waiting: Mutex<HashMap<i64, (ScheduleId, AgentId)>>,
}

/// The schedule module's side of the office: starts the runs of due times and
/// of Run now, settles the directives that waited, and switches off the
/// schedules whose target is gone. Clones share one state.
#[derive(Clone)]
pub struct ScheduleBridge {
    // Shared, so the server's own state holds a pointer and not the parts.
    pub(super) parts: Arc<ScheduleParts>,
    pub(super) book: RunBook,
    state: Arc<BridgeState>,
}

impl ScheduleBridge {
    #[must_use]
    pub fn new(parts: ScheduleParts) -> Self {
        let book = RunBook::new(Arc::clone(&parts.store));
        Self {
            parts: Arc::new(parts),
            book,
            state: Arc::default(),
        }
    }

    /// The lock of the read-then-write of schedules; hold it for the whole of
    /// one request or round, never across an `await`.
    pub fn lock_schedules(&self) -> MutexGuard<'_, ()> {
        self.state
            .schedules
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts the run of a due time that a round found. The round saved the
    /// handled due time before it calls this, so a crash never repeats the
    /// run (0025 D17).
    pub fn start_due(&self, schedule: Schedule, run: DueRun) {
        self.launch(schedule, run.due, RunVia::Due);
    }

    /// Opens the run and hands it to its own task, or records the skip of an
    /// overlapping run at once.
    fn launch(&self, schedule: Schedule, due: Timestamp, via: RunVia) {
        let run = match self.open_run(&schedule, due, via) {
            Ok(Some(run)) => run,
            Ok(None) => return,
            Err(error) => return self.report_storage(&error),
        };
        let bridge = self.clone();
        self.parts.runtime.spawn(async move {
            bridge.drive(&schedule, run).await;
        });
    }

    /// Opens a run row, or records the run as skipped when the schedule's
    /// previous run has no result or still waits (FR-008).
    fn open_run(
        &self,
        schedule: &Schedule,
        due: Timestamp,
        via: RunVia,
    ) -> Result<Option<RunId>, ScheduleApplicationError> {
        let _start = self
            .state
            .starts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.parts.labels.remember(schedule.id, &schedule.name);
        let started = self.parts.time.now();
        let Some(reason) = self.book.skip_reason(schedule.id)? else {
            let open = NewRun {
                schedule_id: schedule.id,
                due,
                started,
                via,
                outcome: None,
            };
            return self.book.open(&open).map(Some);
        };
        let skipped = NewRun {
            schedule_id: schedule.id,
            due,
            started,
            via,
            outcome: Some(RunOutcome {
                result: RunResult::Skipped,
                reason: Some(reason.to_owned()),
                queue_entry: None,
            }),
        };
        self.book.open(&skipped)?;
        self.record_entry(schedule, AgentAction::Directive, Some(WAITING_REASON));
        self.publish_changed(schedule.id);
        Ok(None)
    }

    /// Records one missed entry for the due times of one schedule that a
    /// round found missed: a history row and one action record entry in the
    /// target's record (FR-006, D5).
    pub fn record_missed(
        &self,
        schedule: &Schedule,
        batch: &MissedBatch,
        now: Timestamp,
        zone: &TimeZone,
    ) {
        let reason = missed_reason(batch, zone);
        let row = NewRun {
            schedule_id: schedule.id,
            due: batch.last,
            started: now,
            via: RunVia::Due,
            outcome: Some(RunOutcome {
                result: RunResult::Missed,
                reason: Some(reason.clone()),
                queue_entry: None,
            }),
        };
        if let Err(error) = self.book.open(&row) {
            self.report_storage(&error);
        }
        self.parts.labels.remember(schedule.id, &schedule.name);
        self.record_entry(schedule, AgentAction::Directive, Some(&reason));
    }

    /// Switches off, with the off reason, every schedule whose target no
    /// longer exists, and notifies once (FR-010). It runs after each delete,
    /// a subtree delete and the end of a temp included, and is idempotent.
    pub fn forget_removed_targets(&self) {
        let _locked = self.lock_schedules();
        let schedules = match self.parts.store.list() {
            Ok(schedules) => schedules,
            Err(error) => return self.report_storage(&error),
        };
        for schedule in schedules
            .iter()
            .filter(|schedule| schedule.off_reason != Some(OffReason::TargetDeleted))
            .filter(|schedule| self.target_is_gone(schedule.target))
        {
            self.switch_off_for_removed_target(schedule);
        }
    }

    /// Switches one schedule off because its target is gone. The caller holds
    /// the schedule lock.
    pub(super) fn switch_off_for_removed_target(&self, schedule: &Schedule) {
        let mut off = schedule.clone();
        let was_on = off.switched_on;
        off.switched_on = false;
        off.next_due = None;
        off.off_reason = Some(OffReason::TargetDeleted);
        if let Err(error) = self.parts.store.save(&off, self.parts.cap.cap()) {
            return self.report_storage(&error);
        }
        self.publish_changed(off.id);
        if was_on {
            self.notify_once(&off, TARGET_REMOVED_NOTICE, None);
        }
    }

    fn target_is_gone(&self, target: AgentId) -> bool {
        matches!(self.parts.agents.find(target), Ok(None))
    }

    /// Sends the notification of a result that FR-011 names, unless one went
    /// out since the schedule's last successful run, and remembers it. `about`
    /// is the target the row names, or `None` when the target is deleted. The
    /// caller holds the schedule lock.
    pub(super) fn notify_once(&self, schedule: &Schedule, detail: &str, about: Option<AgentId>) {
        let Ok(Some(mut current)) = self.parts.store.get(schedule.id) else {
            return;
        };
        if current.notified_since_success {
            return;
        }
        self.parts
            .notifications
            .record_schedule_result(&current.name, about, detail.to_owned());
        current.notified_since_success = true;
        self.save_quietly(&current);
    }

    /// A successful run lets the next failure notify again. The caller holds
    /// the schedule lock.
    pub(super) fn clear_notified(&self, schedule_id: ScheduleId) {
        let Ok(Some(mut current)) = self.parts.store.get(schedule_id) else {
            return;
        };
        if current.notified_since_success {
            current.notified_since_success = false;
            self.save_quietly(&current);
        }
    }

    fn save_quietly(&self, schedule: &Schedule) {
        match self.parts.store.save(schedule, self.parts.cap.cap()) {
            Ok(()) => {}
            Err(error) => self.report_storage(&error),
        }
    }

    /// Writes one action record entry in the target's record with the
    /// schedule as actor, and publishes it.
    pub(super) fn record_entry(
        &self,
        schedule: &Schedule,
        action: AgentAction,
        failure: Option<&str>,
    ) {
        drop(take_recorded_events());
        self.write_entry(
            schedule.id,
            &schedule.name,
            schedule.target,
            action,
            failure,
        );
        for event in take_recorded_events() {
            self.parts.events.publish(&event);
        }
    }

    /// As `record_entry`, without taking the recorded events: for a caller
    /// that is inside another record-and-publish, which publishes them all.
    fn write_entry(
        &self,
        schedule_id: ScheduleId,
        name: &str,
        target: AgentId,
        action: AgentAction,
        failure: Option<&str>,
    ) {
        let _actor = ActorScope::enter_labeled(Actor::Schedule(schedule_id), name);
        if let Err(error) = self.parts.journal.record(target, action, failure) {
            self.parts
                .alerts
                .report_disk_failure(&format!("the action record failed: {error}"));
        }
    }

    pub fn publish_changed(&self, schedule_id: ScheduleId) {
        if let Ok(Some(schedule)) = self.parts.store.get(schedule_id) {
            self.parts
                .events
                .publish(&schedule_changed_event(&schedule));
        }
    }

    pub fn report_storage(&self, error: &ScheduleApplicationError) {
        let message = format!("the schedule store failed: {error}");
        if error.is_disk_failure() {
            self.parts.alerts.report_disk_failure(&message);
        } else {
            self.parts.log.error(&message);
        }
    }

    pub(super) fn register_waiting(&self, run: RunId, schedule: ScheduleId, target: AgentId) {
        self.state
            .waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(run.0, (schedule, target));
    }

    pub(super) fn forget_waiting(&self, run: RunId) {
        self.state
            .waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&run.0);
    }

    fn take_waiting(&self, run: i64) -> Option<(ScheduleId, AgentId)> {
        self.state
            .waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&run)
    }
}

impl TargetCheck for ScheduleBridge {
    fn exists(&self, agent_id: AgentId) -> bool {
        // A failed read is no proof that the agent is gone; the run checks
        // again before it acts.
        !self.target_is_gone(agent_id)
    }
}

impl RunNow for ScheduleBridge {
    fn run_now(&self, schedule: &Schedule) -> Result<(), String> {
        let now = self.parts.time.now();
        self.launch(schedule.clone(), now, RunVia::Now);
        Ok(())
    }
}

impl ScheduledOutcomes for ScheduleBridge {
    /// The queue's word on a scheduled directive that had waited. It runs on
    /// the delivery thread, inside the queue's own record-and-publish.
    fn settled(&self, run_id: i64, outcome: ScheduledOutcome) {
        let Some((schedule_id, target)) = self.take_waiting(run_id) else {
            return;
        };
        let label = self.parts.labels.get(schedule_id);
        let name = label.as_deref().unwrap_or_default();
        let (result, reason) = match outcome {
            ScheduledOutcome::Submitted => {
                self.write_entry(schedule_id, name, target, AgentAction::Directive, None);
                (RunResult::Submitted, None)
            }
            ScheduledOutcome::Unsubmitted(reason) => (RunResult::Unsubmitted, Some(reason)),
            ScheduledOutcome::Withdrawn(reason) => (RunResult::Withdrawn, Some(reason)),
        };
        let _locked = self.lock_schedules();
        let settled = RunOutcome {
            result,
            reason: reason.clone(),
            queue_entry: None,
        };
        if let Err(error) = self.book.settle(RunId(run_id), &settled) {
            // A deleted schedule took its history with it; nothing to settle.
            if !matches!(error, ScheduleApplicationError::NotFound(_)) {
                self.report_storage(&error);
            }
            return;
        }
        self.after_result(schedule_id, result, reason.as_deref());
    }
}

impl ScheduleBridge {
    /// The notification and the schedule's flag that follow a final result.
    /// The caller holds the schedule lock.
    pub(super) fn after_result(
        &self,
        schedule_id: ScheduleId,
        result: RunResult,
        reason: Option<&str>,
    ) {
        if result == RunResult::Submitted {
            self.clear_notified(schedule_id);
        } else if notifies(result, reason) {
            self.notify_for_result(schedule_id, result, reason);
        }
        self.publish_changed(schedule_id);
    }

    fn notify_for_result(&self, schedule_id: ScheduleId, result: RunResult, reason: Option<&str>) {
        let Ok(Some(schedule)) = self.parts.store.get(schedule_id) else {
            return;
        };
        let target = self
            .parts
            .agents
            .find(schedule.target)
            .ok()
            .flatten()
            .map_or_else(|| "the target".to_owned(), |agent| agent.name.to_string());
        let detail = match reason {
            Some(reason) => format!("{target}: {}: {reason}", result.as_str()),
            None => format!("{target}: {}", result.as_str()),
        };
        self.notify_once(&schedule, &detail, Some(schedule.target));
    }
}

/// True for the results that FR-011 tells the person about. A hold refusal is
/// told apart before this call; a withdrawal or a failure at a server stop is
/// not news, because the person stopped the server.
fn notifies(result: RunResult, reason: Option<&str>) -> bool {
    match result {
        RunResult::Skipped | RunResult::Refused | RunResult::Unsubmitted => true,
        RunResult::Failed => reason != Some(SETTLED_AT_START_REASON),
        RunResult::Withdrawn => reason != Some(SERVER_STOPPED),
        RunResult::Submitted | RunResult::Waiting | RunResult::Missed => false,
    }
}

fn missed_reason(batch: &MissedBatch, zone: &TimeZone) -> String {
    let runs = match batch.count {
        1 => "1 run missed".to_owned(),
        count => format!("{count} runs missed"),
    };
    format!(
        "{runs}, first {}, last {}",
        local_text(batch.first, zone),
        local_text(batch.last, zone)
    )
}

/// A moment in the zone's wall-clock time, as `2026-11-23 02:00`.
fn local_text(moment: Timestamp, zone: &TimeZone) -> String {
    jiff::Timestamp::from_millisecond(moment.as_unix_millis()).map_or_else(
        |_| moment.as_unix_millis().to_string(),
        |instant| {
            instant
                .to_zoned(zone.clone())
                .strftime("%Y-%m-%d %H:%M")
                .to_string()
        },
    )
}

#[cfg(test)]
#[path = "schedule_bridge.tests.rs"]
mod tests;
