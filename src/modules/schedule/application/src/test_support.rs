// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-memory fakes of the ports, for the use case tests.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use jiff::civil::time;
use jiff::tz::TimeZone;
use kernel::{AgentId, ScheduleId, Timestamp};
use schedule_domain::{Repeat, Schedule, ScheduleCap, ZoneSource};

use crate::common::errors::ScheduleApplicationError;
use crate::common::interfaces::{ScheduleCapSource, ScheduleClock, ScheduleStore, TargetCheck};
use crate::common::models::{HISTORY_LIMIT, NewRun, RunId, RunOutcome, RunResult, ScheduleRun};
use crate::services::ScheduleService;

#[derive(Default)]
struct Rows {
    schedules: Vec<Schedule>,
    runs: Vec<ScheduleRun>,
    last_run_id: i64,
}

#[derive(Default)]
pub struct MemoryStore(Mutex<Rows>);

impl MemoryStore {
    fn rows(&self) -> MutexGuard<'_, Rows> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ScheduleStore for MemoryStore {
    fn list(&self) -> Result<Vec<Schedule>, ScheduleApplicationError> {
        Ok(self.rows().schedules.clone())
    }

    fn get(&self, id: ScheduleId) -> Result<Option<Schedule>, ScheduleApplicationError> {
        Ok(self
            .rows()
            .schedules
            .iter()
            .find(|row| row.id == id)
            .cloned())
    }

    fn save(&self, schedule: &Schedule, cap: ScheduleCap) -> Result<(), ScheduleApplicationError> {
        let mut rows = self.rows();
        let key = schedule.name.to_lowercase();
        if rows
            .schedules
            .iter()
            .any(|row| row.id != schedule.id && row.name.to_lowercase() == key)
        {
            return Err(ScheduleApplicationError::NameTaken {
                name: schedule.name.clone(),
            });
        }
        if let Some(row) = rows.schedules.iter_mut().find(|row| row.id == schedule.id) {
            *row = schedule.clone();
        } else {
            if rows.schedules.len() >= cap.get() as usize {
                return Err(ScheduleApplicationError::CapReached { cap: cap.get() });
            }
            rows.schedules.push(schedule.clone());
        }
        Ok(())
    }

    fn delete(&self, id: ScheduleId) -> Result<bool, ScheduleApplicationError> {
        let mut rows = self.rows();
        let before = rows.schedules.len();
        rows.schedules.retain(|row| row.id != id);
        rows.runs.retain(|run| run.schedule_id != id);
        Ok(rows.schedules.len() != before)
    }

    fn record_run(&self, run: &NewRun) -> Result<RunId, ScheduleApplicationError> {
        let mut rows = self.rows();
        rows.last_run_id += 1;
        let id = RunId(rows.last_run_id);
        rows.runs.push(ScheduleRun {
            id,
            schedule_id: run.schedule_id,
            due: run.due,
            started: run.started,
            via: run.via,
            outcome: run.outcome.clone(),
        });
        Ok(id)
    }

    fn get_run(&self, id: RunId) -> Result<Option<ScheduleRun>, ScheduleApplicationError> {
        Ok(self.rows().runs.iter().find(|run| run.id == id).cloned())
    }

    fn settle_run(
        &self,
        id: RunId,
        outcome: &RunOutcome,
    ) -> Result<bool, ScheduleApplicationError> {
        let mut rows = self.rows();
        let Some(run) = rows.runs.iter_mut().find(|run| run.id == id) else {
            return Ok(false);
        };
        run.outcome = Some(outcome.clone());
        Ok(true)
    }

    fn history(&self, id: ScheduleId) -> Result<Vec<ScheduleRun>, ScheduleApplicationError> {
        let mut runs: Vec<ScheduleRun> = self
            .rows()
            .runs
            .iter()
            .filter(|run| run.schedule_id == id)
            .cloned()
            .collect();
        runs.reverse();
        runs.truncate(HISTORY_LIMIT);
        Ok(runs)
    }

    fn fail_open_runs(&self, reason: &str) -> Result<usize, ScheduleApplicationError> {
        let mut rows = self.rows();
        let mut count = 0;
        for run in rows.runs.iter_mut().filter(|run| run.outcome.is_none()) {
            run.outcome = Some(RunOutcome {
                result: RunResult::Failed,
                reason: Some(reason.to_owned()),
                queue_entry: None,
            });
            count += 1;
        }
        Ok(count)
    }
}

/// A clock that tests move.
pub struct SetClock(pub Mutex<i64>);

impl SetClock {
    pub fn set(&self, millis: i64) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = millis;
    }
}

impl ScheduleClock for SetClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(*self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

pub struct FixedCap(pub u32);

impl ScheduleCapSource for FixedCap {
    fn cap(&self) -> ScheduleCap {
        ScheduleCap::new(i64::from(self.0)).unwrap_or_default()
    }
}

/// The agents that exist.
pub struct KnownAgents(pub HashSet<AgentId>);

impl TargetCheck for KnownAgents {
    fn exists(&self, agent_id: AgentId) -> bool {
        self.0.contains(&agent_id)
    }
}

pub struct FixedZone(pub TimeZone);

impl ZoneSource for FixedZone {
    fn current(&self) -> TimeZone {
        self.0.clone()
    }
}

/// 2026-06-01 12:00 UTC.
pub const NOW: i64 = 1_780_315_200_000;
pub const HOUR: i64 = 3_600_000;

pub fn target() -> AgentId {
    AgentId::from_u128(42)
}

pub fn utc() -> TimeZone {
    TimeZone::UTC
}

pub struct Fixture {
    pub service: ScheduleService,
    pub store: Arc<MemoryStore>,
    pub clock: Arc<SetClock>,
}

pub fn fixture_with_cap(cap: u32) -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let clock = Arc::new(SetClock(Mutex::new(NOW)));
    let service = ScheduleService::new(
        store.clone(),
        clock.clone(),
        Arc::new(FixedCap(cap)),
        Arc::new(KnownAgents(HashSet::from([target()]))),
        Arc::new(FixedZone(utc())),
    );
    Fixture {
        service,
        store,
        clock,
    }
}

pub fn fixture() -> Fixture {
    fixture_with_cap(50)
}

pub fn daily_at_eight() -> Repeat {
    Repeat::Daily {
        time: time(8, 0, 0, 0),
    }
}
