// SPDX-License-Identifier: AGPL-3.0-or-later

//! The scheduler (0025 D17): every tick it plans the due times of the
//! schedules that are on, saves what changed, records the missed ones, and
//! starts one run per due time. It waits for the sweep of 0017 before its
//! first round.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use kernel::ScheduleId;
use schedule_application::common::errors::ScheduleApplicationError;
use schedule_application::common::interfaces::{ScheduleCapSource, ScheduleStore};
use schedule_application::services::{Plan, RoundPlanner};
use schedule_domain::Schedule;
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
}

impl ScheduleRunner {
    #[must_use]
    pub fn new(parts: RunnerParts) -> Self {
        Self {
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

    /// One round: the plan, the saves, the missed entries, then the runs.
    fn round(&self) {
        let now = self.time.now();
        let zone = self.time.zone();
        let (schedules, plan) = {
            let _locked = self.bridge.lock_schedules();
            let schedules = match self.store.list() {
                Ok(schedules) => schedules,
                Err(error) => return self.report(&error),
            };
            let plan = RoundPlanner::plan(now, &zone, &schedules);
            // The handled due times are stored before any run starts, so a
            // crash never repeats a run (D17).
            if let Err(error) = self.save_changed(&plan) {
                return self.report(&error);
            }
            let by_id: HashMap<ScheduleId, &Schedule> = schedules
                .iter()
                .map(|schedule| (schedule.id, schedule))
                .collect();
            for batch in &plan.missed {
                if let Some(schedule) = by_id.get(&batch.schedule_id) {
                    self.bridge.record_missed(schedule, batch, now, &zone);
                }
            }
            (schedules, plan)
        };
        if !plan.missed.is_empty() {
            let count = u32::try_from(plan.missed.len()).unwrap_or(u32::MAX);
            self.notifications.record_missed_schedules(count, now);
        }
        self.start_runs(&schedules, &plan);
    }

    fn save_changed(&self, plan: &Plan) -> Result<(), ScheduleApplicationError> {
        for changed in &plan.changed {
            self.store.save(changed, self.cap.cap())?;
            self.bridge.publish_changed(changed.id);
        }
        Ok(())
    }

    fn start_runs(&self, schedules: &[Schedule], plan: &Plan) {
        for run in &plan.runs {
            if let Some(schedule) = schedules
                .iter()
                .find(|schedule| schedule.id == run.schedule_id)
            {
                self.bridge.start_due(schedule.clone(), *run);
            }
        }
    }

    fn report(&self, error: &ScheduleApplicationError) {
        self.log
            .error(&format!("a schedule round cannot use the store: {error}"));
    }
}
