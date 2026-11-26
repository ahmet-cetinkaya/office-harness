// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use jiff::civil::Date;
use kernel::{AgentId, ScheduleId, Timestamp};
use rusqlite::ffi::{ErrorCode, SQLITE_CONSTRAINT_UNIQUE};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use schedule_application::common::errors::ScheduleApplicationError;
use schedule_application::common::interfaces::ScheduleStore;
use schedule_application::common::models::{
    HISTORY_LIMIT, NewRun, RunId, RunOutcome, RunResult, RunVia, ScheduleRun,
};
use schedule_domain::{
    OffReason, Repeat, Schedule, ScheduleCap, parse_time, parse_weekday, time_text, weekday_code,
};

/// One shared SQLite connection, the way the host app hands the database to
/// every module (each module keeps its own tables).
pub type SharedHandle = Arc<Mutex<Connection>>;

const SELECT_SCHEDULE: &str = "SELECT id, name, text, target_agent_id, repeat_kind, repeat_date, \
     repeat_time, repeat_days, repeat_hours, switched_on, anchor, next_due, last_handled_due, \
     last_missed, off_reason, notified_since_success FROM schedule_schedules";
const SELECT_RUN: &str = "SELECT id, schedule_id, due, started, via, result, reason, queue_entry \
     FROM schedule_runs";
const UPSERT_SCHEDULE: &str = "INSERT INTO schedule_schedules \
     (id, name, name_key, text, target_agent_id, repeat_kind, repeat_date, repeat_time, \
      repeat_days, repeat_hours, switched_on, anchor, next_due, last_handled_due, last_missed, \
      off_reason, notified_since_success) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17) \
     ON CONFLICT(id) DO UPDATE SET name = ?2, name_key = ?3, text = ?4, target_agent_id = ?5, \
      repeat_kind = ?6, repeat_date = ?7, repeat_time = ?8, repeat_days = ?9, repeat_hours = ?10, \
      switched_on = ?11, anchor = ?12, next_due = ?13, last_handled_due = ?14, last_missed = ?15, \
      off_reason = ?16, notified_since_success = ?17";
const DAY_SEPARATOR: &str = ",";

fn corrupt(field: &'static str, text: &str) -> ScheduleApplicationError {
    ScheduleApplicationError::Storage {
        message: format!("stored data is invalid, {field}: {text}"),
        is_disk_failure: false,
    }
}

fn storage_failure(error: &rusqlite::Error) -> ScheduleApplicationError {
    let is_disk_failure = matches!(
        error.sqlite_error_code(),
        Some(
            ErrorCode::DiskFull
                | ErrorCode::SystemIoFailure
                | ErrorCode::ReadOnly
                | ErrorCode::CannotOpen
        )
    );
    ScheduleApplicationError::Storage {
        message: error.to_string(),
        is_disk_failure,
    }
}

/// The repeat as the columns that hold it.
struct RepeatColumns {
    kind: &'static str,
    date: Option<String>,
    time: Option<String>,
    days: Option<String>,
    hours: Option<i64>,
}

fn encode_repeat(repeat: &Repeat) -> RepeatColumns {
    let none = RepeatColumns {
        kind: "",
        date: None,
        time: None,
        days: None,
        hours: None,
    };
    match repeat {
        Repeat::Once { date, time } => RepeatColumns {
            kind: "once",
            date: Some(date.to_string()),
            time: Some(time_text(*time)),
            ..none
        },
        Repeat::Daily { time } => RepeatColumns {
            kind: "daily",
            time: Some(time_text(*time)),
            ..none
        },
        Repeat::Weekly { days, time } => RepeatColumns {
            kind: "weekly",
            time: Some(time_text(*time)),
            days: Some(
                days.iter()
                    .map(|day| weekday_code(*day))
                    .collect::<Vec<_>>()
                    .join(DAY_SEPARATOR),
            ),
            ..none
        },
        Repeat::EveryHours { hours } => RepeatColumns {
            kind: "every_hours",
            hours: Some(i64::from(*hours)),
            ..none
        },
    }
}

fn decode_repeat(columns: &RepeatColumns, kind: &str) -> Result<Repeat, ScheduleApplicationError> {
    let time = || {
        columns
            .time
            .as_deref()
            .and_then(parse_time)
            .ok_or_else(|| corrupt("repeat time", kind))
    };
    match kind {
        "once" => Ok(Repeat::Once {
            date: columns
                .date
                .as_deref()
                .and_then(|text| text.parse::<Date>().ok())
                .ok_or_else(|| corrupt("repeat date", kind))?,
            time: time()?,
        }),
        "daily" => Ok(Repeat::Daily { time: time()? }),
        "weekly" => Ok(Repeat::Weekly {
            days: columns
                .days
                .as_deref()
                .unwrap_or_default()
                .split(DAY_SEPARATOR)
                .filter(|code| !code.is_empty())
                .map(|code| parse_weekday(code).ok_or_else(|| corrupt("repeat days", code)))
                .collect::<Result<Vec<_>, _>>()?,
            time: time()?,
        }),
        "every_hours" => Ok(Repeat::EveryHours {
            hours: columns
                .hours
                .and_then(|hours| u32::try_from(hours).ok())
                .ok_or_else(|| corrupt("repeat hours", kind))?,
        }),
        other => Err(corrupt("repeat kind", other)),
    }
}

struct ScheduleRow {
    id: String,
    name: String,
    text: String,
    target: String,
    kind: String,
    repeat: RepeatColumns,
    switched_on: bool,
    anchor: i64,
    next_due: Option<i64>,
    last_handled_due: Option<i64>,
    last_missed: Option<i64>,
    off_reason: Option<String>,
    notified_since_success: bool,
}

impl ScheduleRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            text: row.get(2)?,
            target: row.get(3)?,
            kind: row.get(4)?,
            repeat: RepeatColumns {
                kind: "",
                date: row.get(5)?,
                time: row.get(6)?,
                days: row.get(7)?,
                hours: row.get(8)?,
            },
            switched_on: row.get(9)?,
            anchor: row.get(10)?,
            next_due: row.get(11)?,
            last_handled_due: row.get(12)?,
            last_missed: row.get(13)?,
            off_reason: row.get(14)?,
            notified_since_success: row.get(15)?,
        })
    }

    fn into_schedule(self) -> Result<Schedule, ScheduleApplicationError> {
        let off_reason = self
            .off_reason
            .as_deref()
            .map(|text| OffReason::parse(text).ok_or_else(|| corrupt("off reason", text)))
            .transpose()?;
        let millis = |value: Option<i64>| value.map(Timestamp::from_unix_millis);
        Ok(Schedule {
            id: self
                .id
                .parse::<ScheduleId>()
                .map_err(|error| corrupt("schedule id", &error.to_string()))?,
            name: self.name,
            text: self.text,
            target: self
                .target
                .parse::<AgentId>()
                .map_err(|error| corrupt("target", &error.to_string()))?,
            repeat: decode_repeat(&self.repeat, &self.kind)?,
            switched_on: self.switched_on,
            anchor: Timestamp::from_unix_millis(self.anchor),
            next_due: millis(self.next_due),
            last_handled_due: millis(self.last_handled_due),
            last_missed: millis(self.last_missed),
            off_reason,
            notified_since_success: self.notified_since_success,
        })
    }
}

struct RunRow {
    id: i64,
    schedule_id: String,
    due: i64,
    started: i64,
    via: String,
    result: Option<String>,
    reason: Option<String>,
    queue_entry: Option<String>,
}

impl RunRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            schedule_id: row.get(1)?,
            due: row.get(2)?,
            started: row.get(3)?,
            via: row.get(4)?,
            result: row.get(5)?,
            reason: row.get(6)?,
            queue_entry: row.get(7)?,
        })
    }

    fn into_run(self) -> Result<ScheduleRun, ScheduleApplicationError> {
        let outcome = self
            .result
            .as_deref()
            .map(|text| {
                RunResult::parse(text)
                    .map(|result| RunOutcome {
                        result,
                        reason: self.reason.clone(),
                        queue_entry: self.queue_entry.clone(),
                    })
                    .ok_or_else(|| corrupt("run result", text))
            })
            .transpose()?;
        Ok(ScheduleRun {
            id: RunId(self.id),
            schedule_id: self
                .schedule_id
                .parse::<ScheduleId>()
                .map_err(|error| corrupt("schedule id", &error.to_string()))?,
            due: Timestamp::from_unix_millis(self.due),
            started: Timestamp::from_unix_millis(self.started),
            via: RunVia::parse(&self.via).ok_or_else(|| corrupt("run via", &self.via))?,
            outcome,
        })
    }
}

/// The schedules and runs in the `schedule_schedules` and `schedule_runs`
/// tables.
#[derive(Clone)]
pub struct ScheduleRepository {
    connection: SharedHandle,
}

impl ScheduleRepository {
    #[must_use]
    pub fn new(connection: SharedHandle) -> Self {
        Self { connection }
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        // A panic in another call leaves SQLite consistent, so a poisoned
        // lock is safe to reuse.
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

fn is_unique_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _) if failure.extended_code == SQLITE_CONSTRAINT_UNIQUE
    )
}

fn save_schedule(
    connection: &mut Connection,
    schedule: &Schedule,
    cap: ScheduleCap,
) -> rusqlite::Result<Result<(), ScheduleApplicationError>> {
    // The write lock is taken at the start, so two saves at the cap run one
    // after the other and the second sees the first one's row.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let id = schedule.id.to_string();
    let name_key = schedule.name.to_lowercase();
    let name_taken: bool = transaction.query_row(
        "SELECT EXISTS (SELECT 1 FROM schedule_schedules WHERE name_key = ?1 AND id <> ?2)",
        params![name_key, id],
        |row| row.get(0),
    )?;
    if name_taken {
        return Ok(Err(ScheduleApplicationError::NameTaken {
            name: schedule.name.clone(),
        }));
    }
    let exists: bool = transaction.query_row(
        "SELECT EXISTS (SELECT 1 FROM schedule_schedules WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?;
    if !exists {
        let count: i64 =
            transaction.query_row("SELECT COUNT(*) FROM schedule_schedules", [], |row| {
                row.get(0)
            })?;
        if count >= i64::from(cap.get()) {
            return Ok(Err(ScheduleApplicationError::CapReached { cap: cap.get() }));
        }
    }
    let repeat = encode_repeat(&schedule.repeat);
    let millis = |value: Option<Timestamp>| value.map(Timestamp::as_unix_millis);
    transaction.execute(
        UPSERT_SCHEDULE,
        params![
            id,
            schedule.name,
            name_key,
            schedule.text,
            schedule.target.to_string(),
            repeat.kind,
            repeat.date,
            repeat.time,
            repeat.days,
            repeat.hours,
            schedule.switched_on,
            schedule.anchor.as_unix_millis(),
            millis(schedule.next_due),
            millis(schedule.last_handled_due),
            millis(schedule.last_missed),
            schedule.off_reason.map(OffReason::as_str),
            schedule.notified_since_success,
        ],
    )?;
    transaction.commit()?;
    Ok(Ok(()))
}

fn insert_run(connection: &mut Connection, run: &NewRun) -> rusqlite::Result<RunId> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let schedule_id = run.schedule_id.to_string();
    let outcome = run.outcome.as_ref();
    transaction.execute(
        "INSERT INTO schedule_runs (schedule_id, due, started, via, result, reason, queue_entry) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            schedule_id,
            run.due.as_unix_millis(),
            run.started.as_unix_millis(),
            run.via.as_str(),
            outcome.map(|outcome| outcome.result.as_str()),
            outcome.and_then(|outcome| outcome.reason.as_deref()),
            outcome.and_then(|outcome| outcome.queue_entry.as_deref()),
        ],
    )?;
    let id = transaction.last_insert_rowid();
    transaction.execute(
        "DELETE FROM schedule_runs WHERE schedule_id = ?1 \
         AND result IS NOT NULL AND result <> ?3 AND id NOT IN \
         (SELECT id FROM schedule_runs WHERE schedule_id = ?1 ORDER BY id DESC LIMIT ?2)",
        params![
            schedule_id,
            i64::try_from(HISTORY_LIMIT).unwrap_or(i64::MAX),
            RunResult::Waiting.as_str()
        ],
    )?;
    transaction.commit()?;
    Ok(RunId(id))
}

fn delete_schedule(connection: &mut Connection, id: ScheduleId) -> rusqlite::Result<bool> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let id = id.to_string();
    transaction.execute(
        "DELETE FROM schedule_runs WHERE schedule_id = ?1",
        params![id],
    )?;
    let removed =
        transaction.execute("DELETE FROM schedule_schedules WHERE id = ?1", params![id])?;
    transaction.commit()?;
    Ok(removed > 0)
}

impl ScheduleStore for ScheduleRepository {
    fn list(&self) -> Result<Vec<Schedule>, ScheduleApplicationError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(&format!("{SELECT_SCHEDULE} ORDER BY name_key"))
            .map_err(|error| storage_failure(&error))?;
        let rows = statement
            .query_map([], ScheduleRow::read)
            .map_err(|error| storage_failure(&error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| storage_failure(&error))?;
        rows.into_iter().map(ScheduleRow::into_schedule).collect()
    }

    fn get(&self, id: ScheduleId) -> Result<Option<Schedule>, ScheduleApplicationError> {
        let row = self
            .connection()
            .query_row(
                &format!("{SELECT_SCHEDULE} WHERE id = ?1"),
                params![id.to_string()],
                ScheduleRow::read,
            )
            .optional()
            .map_err(|error| storage_failure(&error))?;
        row.map(ScheduleRow::into_schedule).transpose()
    }

    fn save(&self, schedule: &Schedule, cap: ScheduleCap) -> Result<(), ScheduleApplicationError> {
        match save_schedule(&mut self.connection(), schedule, cap) {
            Ok(outcome) => outcome,
            Err(error) if is_unique_violation(&error) => Err(ScheduleApplicationError::NameTaken {
                name: schedule.name.clone(),
            }),
            Err(error) => Err(storage_failure(&error)),
        }
    }

    fn delete(&self, id: ScheduleId) -> Result<bool, ScheduleApplicationError> {
        delete_schedule(&mut self.connection(), id).map_err(|error| storage_failure(&error))
    }

    fn record_run(&self, run: &NewRun) -> Result<RunId, ScheduleApplicationError> {
        insert_run(&mut self.connection(), run).map_err(|error| storage_failure(&error))
    }

    fn get_run(&self, id: RunId) -> Result<Option<ScheduleRun>, ScheduleApplicationError> {
        let row = self
            .connection()
            .query_row(
                &format!("{SELECT_RUN} WHERE id = ?1"),
                params![id.0],
                RunRow::read,
            )
            .optional()
            .map_err(|error| storage_failure(&error))?;
        row.map(RunRow::into_run).transpose()
    }

    fn settle_run(
        &self,
        id: RunId,
        outcome: &RunOutcome,
    ) -> Result<bool, ScheduleApplicationError> {
        let changed = self
            .connection()
            .execute(
                "UPDATE schedule_runs SET result = ?2, reason = ?3, queue_entry = ?4 \
                 WHERE id = ?1 AND (result IS NULL OR (result = ?5 AND ?6))",
                params![
                    id.0,
                    outcome.result.as_str(),
                    outcome.reason,
                    outcome.queue_entry,
                    RunResult::Waiting.as_str(),
                    outcome.result.settles_a_waiting_run()
                ],
            )
            .map_err(|error| storage_failure(&error))?;
        Ok(changed > 0)
    }

    fn history(&self, id: ScheduleId) -> Result<Vec<ScheduleRun>, ScheduleApplicationError> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(&format!(
                "{SELECT_RUN} WHERE schedule_id = ?1 ORDER BY id DESC LIMIT ?2"
            ))
            .map_err(|error| storage_failure(&error))?;
        let rows = statement
            .query_map(
                params![
                    id.to_string(),
                    i64::try_from(HISTORY_LIMIT).unwrap_or(i64::MAX)
                ],
                RunRow::read,
            )
            .map_err(|error| storage_failure(&error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| storage_failure(&error))?;
        rows.into_iter().map(RunRow::into_run).collect()
    }

    fn fail_open_runs(&self, reason: &str) -> Result<usize, ScheduleApplicationError> {
        self.connection()
            .execute(
                "UPDATE schedule_runs SET result = ?1, reason = ?2 \
                 WHERE result IS NULL OR result = ?3",
                params![
                    RunResult::Failed.as_str(),
                    reason,
                    RunResult::Waiting.as_str()
                ],
            )
            .map_err(|error| storage_failure(&error))
    }
}

#[cfg(test)]
#[path = "schedule_repository.tests.rs"]
mod tests;
