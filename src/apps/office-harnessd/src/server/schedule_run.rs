// SPDX-License-Identifier: AGPL-3.0-or-later

//! The scheduler (0025 D17): every tick it plans the due times of the
//! schedules that are on, saves what changed, records the missed ones, and
//! starts one run per due time. It waits for the sweep of 0017 before its
//! first round.

use std::sync::Arc;
use std::time::Duration;

use kernel::Timestamp;
use schedule_application::common::errors::ScheduleApplicationError;
use schedule_application::common::interfaces::{ScheduleCapSource, ScheduleStore};
use schedule_application::services::{Plan, RoundPlanner};
use tokio::sync::watch;
use tokio::task::spawn_blocking;
use tokio::time::{MissedTickBehavior, interval};

use crate::bootstrap::ServerLog;
use crate::bridges::{NotificationBridge, ScheduleBridge, ScheduleTime};
use crate::sweep::{SweepGate, SweepState};

/// What the scheduler is built from.
pub struct RunnerParts {
    pub bridge: ScheduleBridge,
    pub store: Arc<dyn ScheduleStore>,
    pub cap: Arc<dyn ScheduleCapSource>,
    pub notifications: NotificationBridge,
    pub time: &'static dyn ScheduleTime,
    pub gate: SweepGate,
    pub tick: Duration,
    pub log: ServerLog,
}

/// The scheduler task. Clones share the bridge and the store.
#[derive(Clone)]
pub struct ScheduleRunner {
    bridge: ScheduleBridge,
    store: Arc<dyn ScheduleStore>,
    cap: Arc<dyn ScheduleCapSource>,
    notifications: NotificationBridge,
    time: &'static dyn ScheduleTime,
    gate: SweepGate,
    tick: Duration,
    log: ServerLog,
    /// The moment the scheduler was built: a due time before it is missed.
    started: Timestamp,
}

impl ScheduleRunner {
    #[must_use]
    pub fn new(parts: RunnerParts) -> Self {
        Self {
            started: parts.time.now(),
            bridge: parts.bridge,
            store: parts.store,
            cap: parts.cap,
            notifications: parts.notifications,
            time: parts.time,
            gate: parts.gate,
            tick: parts.tick,
            log: parts.log,
        }
    }

    /// Runs until the server stops: `stopping` turns true or its sender is
    /// gone. Waits for the sweep before the first round, and skips a round
    /// while the sweep runs (FR-005).
    pub fn spawn(self, mut stopping: watch::Receiver<bool>) {
        tokio::spawn(async move {
            tokio::select! {
                swept = self.gate.wait_until_done() => if swept.is_err() { return },
                _ = stopping.wait_for(|stopped| *stopped) => return,
            }
            let mut ticks = interval(self.tick);
            ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = ticks.tick() => {}
                    _ = stopping.wait_for(|stopped| *stopped) => return,
                }
                if self.gate.state() == SweepState::Running {
                    continue;
                }
                let runner = self.clone();
                // Every read and write blocks, so a round leaves the async
                // threads.
                if let Err(error) = spawn_blocking(move || runner.round()).await {
                    self.log.error(&format!("a schedule round failed: {error}"));
                }
            }
        });
    }

    /// One round: for each schedule that is on, the plan, the save, and the
    /// missed entries; then the runs. A schedule whose save fails is left as
    /// it was, so its due times are planned again at the next tick, and the
    /// other schedules go on.
    fn round(&self) {
        let now = self.time.now();
        let zone = self.time.zone();
        let mut missed_schedules = 0_u32;
        let mut starts = Vec::new();
        {
            let _locked = self.bridge.lock_schedules();
            let schedules = match self.store.list() {
                Ok(schedules) => schedules,
                Err(error) => return self.bridge.report_storage(&error),
            };
            for schedule in &schedules {
                let plan =
                    RoundPlanner::plan(now, self.started, &zone, std::slice::from_ref(schedule));
                // The handled due times are stored before any run starts, so a
                // crash never repeats a run (D17).
                if let Err(error) = self.save_changed(&plan) {
                    self.bridge.report_storage(&error);
                    continue;
                }
                for batch in &plan.missed {
                    self.bridge.record_missed(schedule, batch, now, &zone);
                }
                missed_schedules += u32::try_from(plan.missed.len()).unwrap_or(u32::MAX);
                starts.extend(plan.runs.into_iter().map(|run| (schedule.clone(), run)));
            }
        }
        if missed_schedules > 0 {
            self.notifications
                .record_missed_schedules(missed_schedules, now);
        }
        for (schedule, run) in starts {
            self.bridge.start_due(schedule, run);
        }
    }

    fn save_changed(&self, plan: &Plan) -> Result<(), ScheduleApplicationError> {
        for changed in &plan.changed {
            self.store.save(changed, self.cap.cap())?;
            self.bridge.publish_changed(changed.id);
        }
        Ok(())
    }
}
