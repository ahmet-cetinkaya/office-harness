// SPDX-License-Identifier: AGPL-3.0-or-later

//! One run of a schedule, from the open run row to its result (0025/FR-007 to
//! FR-011): the target's state, the hold check, the wake, the queue, and the
//! record of what happened.

use std::time::Duration;

use agent_application::common::actor_scope::ActorScope;
use agent_application::common::models::AgentAction;
use agent_domain::enums::AgentStatus;
use hierarchy_application::HierarchyApplicationError;
use hierarchy_application::commands::send_directive::{DirectiveOutcome, ScheduledDirective};
use kernel::{Actor, AgentId};
use schedule_application::common::errors::ScheduleApplicationError;
use schedule_application::common::models::{RunId, RunOutcome, RunResult};
use schedule_application::services::SETTLED_AT_START_REASON;
use schedule_domain::Schedule;
use tokio::task::spawn_blocking;
use tokio::time::sleep;

use super::{ScheduleBridge, take_recorded_events};
use crate::server::WakeResult;

/// How often a run that waits for its target to fall asleep looks again.
const GOING_TO_SLEEP_POLL: Duration = Duration::from_millis(250);
/// The longest a run waits for a target that is going to sleep, as long as a
/// directive may wait in a queue (0012/FR-005).
const GOING_TO_SLEEP_LIMIT: Duration = Duration::from_secs(600);
const STILL_GOING_TO_SLEEP: &str = "still going to sleep";
const WAKE_DID_NOT_END: &str = "the wake did not end in time";
const NOT_SHOWN_AT_INPUT_LINE: &str = "the text did not show at the input line";
const TARGET_GONE_REASON: &str = "the target no longer exists";

/// What a run did, before it is written to the history.
struct Ended {
    result: RunResult,
    reason: Option<String>,
    queue_entry: Option<String>,
    /// A hold refused the run: the person set the hold, so no notification.
    is_hold_refusal: bool,
}

impl Ended {
    fn with_reason(result: RunResult, reason: impl Into<String>) -> Self {
        Self {
            result,
            reason: Some(reason.into()),
            queue_entry: None,
            is_hold_refusal: false,
        }
    }

    fn skipped(reason: impl Into<String>) -> Self {
        Self::with_reason(RunResult::Skipped, reason)
    }

    fn failed(reason: impl Into<String>) -> Self {
        Self::with_reason(RunResult::Failed, reason)
    }

    fn refused_by_hold(reason: String) -> Self {
        Self {
            is_hold_refusal: true,
            ..Self::with_reason(RunResult::Refused, reason)
        }
    }

    fn submitted() -> Self {
        Self {
            result: RunResult::Submitted,
            reason: None,
            queue_entry: None,
            is_hold_refusal: false,
        }
    }

    fn waiting(position: usize) -> Self {
        Self {
            queue_entry: Some(position.to_string()),
            ..Self::with_reason(RunResult::Waiting, format!("place {position} in the queue"))
        }
    }
}

/// What the run finds when it looks at its target.
enum TargetState {
    Gone,
    NotRunning(String),
    GoingToSleep,
    Asleep,
    Running,
    Unreadable(String),
}

impl ScheduleBridge {
    /// Runs the open run to its result and writes the result. The run keeps
    /// the values of the schedule it started with (edge case: an edit while
    /// the run waits).
    pub(super) async fn drive(&self, schedule: &Schedule, run: RunId) {
        let mut stopping = self.parts.stopping.clone();
        let ended = tokio::select! {
            ended = self.walk(schedule, run) => ended,
            _ = stopping.wait_for(|stopped| *stopped) => Ended::failed(SETTLED_AT_START_REASON),
        };
        let owned = schedule.clone();
        let concluded = self
            .blocking(move |bridge| bridge.conclude(&owned, run, &ended))
            .await;
        if let Err(message) = concluded {
            self.parts.log.error(&message);
        }
    }

    async fn blocking<T, F>(&self, work: F) -> Result<T, String>
    where
        F: FnOnce(&Self) -> T + Send + 'static,
        T: Send + 'static,
    {
        let bridge = self.clone();
        spawn_blocking(move || work(&bridge))
            .await
            .map_err(|error| format!("the run task failed: {error}"))
    }

    /// Follows the target's state to a result: skip, wait for sleep, wake, or
    /// queue.
    async fn walk(&self, schedule: &Schedule, run: RunId) -> Ended {
        let target = schedule.target;
        let mut waited = Duration::ZERO;
        loop {
            let state = match self
                .blocking(move |bridge| bridge.target_state(target))
                .await
            {
                Ok(state) => state,
                Err(message) => return Ended::failed(message),
            };
            match state {
                TargetState::Gone => return self.target_gone(schedule).await,
                TargetState::NotRunning(status) => return Ended::skipped(status),
                TargetState::Unreadable(message) => return Ended::failed(message),
                TargetState::Running => return self.queue(schedule, run).await,
                TargetState::Asleep => return self.wake_then_queue(schedule, run).await,
                TargetState::GoingToSleep if waited >= GOING_TO_SLEEP_LIMIT => {
                    return Ended::failed(STILL_GOING_TO_SLEEP);
                }
                TargetState::GoingToSleep => {
                    sleep(GOING_TO_SLEEP_POLL).await;
                    waited += GOING_TO_SLEEP_POLL;
                }
            }
        }
    }

    fn target_state(&self, target: AgentId) -> TargetState {
        match self.parts.agents.find(target) {
            Err(error) => TargetState::Unreadable(error.to_string()),
            Ok(None) => TargetState::Gone,
            Ok(Some(agent)) => match agent.status {
                AgentStatus::Stopped
                | AgentStatus::Exited
                | AgentStatus::Failed
                | AgentStatus::Interrupted => TargetState::NotRunning(agent.status.to_string()),
                AgentStatus::Asleep => TargetState::Asleep,
                AgentStatus::Starting | AgentStatus::Working | AgentStatus::WaitingForInput
                    if self.parts.sleeping.contains(target) =>
                {
                    TargetState::GoingToSleep
                }
                AgentStatus::Starting | AgentStatus::Working | AgentStatus::WaitingForInput => {
                    TargetState::Running
                }
            },
        }
    }

    /// The pre-run existence check (D13): a missed removal hook cannot make a
    /// run against a missing agent.
    async fn target_gone(&self, schedule: &Schedule) -> Ended {
        let (id, target) = (schedule.id, schedule.target);
        let switched = self
            .blocking(move |bridge| {
                let _locked = bridge.lock_schedules();
                let current = bridge.parts.store.get(id).ok().flatten();
                if let Some(current) = current.filter(|current| current.target == target) {
                    bridge.switch_off_for_removed_target(&current);
                }
            })
            .await;
        if let Err(message) = switched {
            self.parts.log.error(&message);
        }
        Ended::skipped(TARGET_GONE_REASON)
    }

    /// An asleep target: the hold first, then the wake as the schedule, then
    /// the directive when the agent runs again (FR-010).
    async fn wake_then_queue(&self, schedule: &Schedule, run: RunId) -> Ended {
        let target = schedule.target;
        let held = self
            .blocking(move |bridge| bridge.hold_refusal(target))
            .await;
        match held {
            Ok(Some(reason)) => return Ended::refused_by_hold(reason),
            Ok(None) => {}
            Err(message) => return Ended::failed(message),
        }
        let woke = self
            .parts
            .resume
            .wake_and_wait(schedule.target, Actor::Schedule(schedule.id))
            .await;
        match woke {
            WakeResult::Continued | WakeResult::NewSession => self.queue(schedule, run).await,
            WakeResult::Held(reason) | WakeResult::Refused(reason) => Ended::failed(reason),
            WakeResult::Unfinished => Ended::failed(WAKE_DID_NOT_END),
        }
    }

    async fn queue(&self, schedule: &Schedule, run: RunId) -> Ended {
        let owned = schedule.clone();
        self.blocking(move |bridge| bridge.queue_blocking(&owned, run))
            .await
            .unwrap_or_else(Ended::failed)
    }

    /// The reason a hold refuses a run on the target now, from the tree as it
    /// is now (FR-007).
    fn hold_refusal(&self, target: AgentId) -> Option<String> {
        match self.parts.tree.store_snapshot() {
            Ok(rows) => self.parts.holds.scheduled_refusal(target, &rows),
            Err(error) => {
                self.parts.log.error(&format!(
                    "a schedule run cannot read the tree for the hold check: {error}"
                ));
                None
            }
        }
    }

    /// The hold check at the start, then the queue, which checks again and
    /// takes the slot of the schedule window.
    fn queue_blocking(&self, schedule: &Schedule, run: RunId) -> Ended {
        if let Some(reason) = self.hold_refusal(schedule.target) {
            return Ended::refused_by_hold(reason);
        }
        let directive = ScheduledDirective {
            schedule: schedule.id,
            target: schedule.target,
            text: schedule.text.clone(),
            run_id: run.0,
        };
        // Registered first: the delivery thread may settle the entry before
        // this call returns.
        self.register_waiting(run, schedule.id, schedule.target);
        let outcome = {
            let _actor = ActorScope::enter_labeled(Actor::Schedule(schedule.id), &*schedule.name);
            drop(take_recorded_events());
            let outcome = self.parts.directives.queue_scheduled(&directive);
            for event in take_recorded_events() {
                self.parts.events.publish(&event);
            }
            outcome
        };
        let ended = match outcome {
            Ok(DirectiveOutcome::Submitted) => Ended::submitted(),
            Ok(DirectiveOutcome::Waiting { position }) => Ended::waiting(position),
            Ok(DirectiveOutcome::Unsubmitted) => {
                Ended::with_reason(RunResult::Unsubmitted, NOT_SHOWN_AT_INPUT_LINE)
            }
            Err(error) => self.ended_by(&error, schedule.target),
        };
        if ended.result != RunResult::Waiting {
            self.forget_waiting(run);
        }
        ended
    }

    fn ended_by(&self, error: &HierarchyApplicationError, target: AgentId) -> Ended {
        match error {
            HierarchyApplicationError::Refused(reason) => Ended {
                is_hold_refusal: self.hold_refusal(target).is_some(),
                ..Ended::with_reason(RunResult::Refused, reason.clone())
            },
            HierarchyApplicationError::Invalid(_) => Ended::skipped(error.to_string()),
            other => Ended::failed(other.to_string()),
        }
    }

    /// Writes the result: the history row, the action record entry, the
    /// notification, and the flag (FR-009, FR-011, FR-012).
    fn conclude(&self, schedule: &Schedule, run: RunId, ended: &Ended) {
        let outcome = RunOutcome {
            result: ended.result,
            reason: ended.reason.clone(),
            queue_entry: ended.queue_entry.clone(),
        };
        match self.book.settle(run, &outcome) {
            // The delivery thread settled a waiting directive first.
            Err(ScheduleApplicationError::RunSettled(_)) => return,
            // A schedule deleted while its run went on has no row to settle;
            // the run still leaves its entry in the target's record.
            Ok(()) | Err(ScheduleApplicationError::NotFound(_)) => {}
            Err(error) => self.report_storage(&error),
        }
        self.record_result(schedule, ended);
        let _locked = self.lock_schedules();
        if ended.is_hold_refusal {
            self.publish_changed(schedule.id);
        } else {
            self.after_result(schedule.id, ended.result, ended.reason.as_deref());
        }
    }

    /// The entry of the run in the target's record. An unsubmitted or a
    /// withdrawn directive was recorded by the queue, once; a waiting one is
    /// recorded when it ends.
    fn record_result(&self, schedule: &Schedule, ended: &Ended) {
        let reason = ended.reason.as_deref();
        match ended.result {
            RunResult::Submitted => self.record_entry(schedule, AgentAction::Directive, None),
            RunResult::Refused => self.record_entry(schedule, AgentAction::Refused, reason),
            RunResult::Skipped | RunResult::Failed => {
                self.record_entry(schedule, AgentAction::Directive, reason);
            }
            RunResult::Waiting
            | RunResult::Unsubmitted
            | RunResult::Withdrawn
            | RunResult::Missed => {}
        }
    }
}
