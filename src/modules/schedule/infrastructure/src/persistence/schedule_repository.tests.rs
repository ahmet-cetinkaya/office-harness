// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::sync::Barrier;
use std::thread;

use jiff::civil::{Weekday, date, time};

use super::*;
use crate::persistence::SCRIPTS;

fn open(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("open");
    connection
        .busy_timeout(std::time::Duration::from_secs(10))
        .expect("timeout");
    let has_tables: bool = connection
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE name = 'schedule_schedules')",
            [],
            |row| row.get(0),
        )
        .expect("probe");
    if !has_tables {
        for script in SCRIPTS {
            connection.execute_batch(script).expect("migrate");
        }
    }
    connection
}

fn repository_at(path: &Path) -> ScheduleRepository {
    ScheduleRepository::new(Arc::new(Mutex::new(open(path))))
}

fn memory_repository() -> ScheduleRepository {
    let connection = Connection::open_in_memory().expect("open");
    for script in SCRIPTS {
        connection.execute_batch(script).expect("migrate");
    }
    ScheduleRepository::new(Arc::new(Mutex::new(connection)))
}

fn cap(value: i64) -> ScheduleCap {
    ScheduleCap::new(value).expect("cap")
}

fn schedule(number: u128, name: &str, repeat: Repeat) -> Schedule {
    Schedule {
        id: ScheduleId::from_u128(number),
        name: name.to_owned(),
        text: "ship the report".to_owned(),
        target: AgentId::from_u128(100 + number),
        repeat,
        switched_on: true,
        anchor: Timestamp::from_unix_millis(1_000),
        next_due: Some(Timestamp::from_unix_millis(5_000)),
        last_handled_due: Some(Timestamp::from_unix_millis(2_000)),
        last_missed: Some(Timestamp::from_unix_millis(1_500)),
        off_reason: None,
        notified_since_success: true,
    }
}

fn hourly(number: u128, name: &str) -> Schedule {
    schedule(number, name, Repeat::EveryHours { hours: 3 })
}

fn run(schedule_number: u128, due: i64, outcome: Option<RunOutcome>) -> NewRun {
    NewRun {
        schedule_id: ScheduleId::from_u128(schedule_number),
        due: Timestamp::from_unix_millis(due),
        started: Timestamp::from_unix_millis(due + 1),
        via: RunVia::Due,
        outcome,
    }
}

fn outcome(result: RunResult) -> RunOutcome {
    RunOutcome {
        result,
        reason: Some("because".to_owned()),
        queue_entry: None,
    }
}

#[test]
fn every_repeat_and_field_survives_a_reopen() {
    let folder = tempfile::tempdir().expect("folder");
    let path = folder.path().join("office.db");
    let mut off = hourly(5, "Off");
    off.switched_on = false;
    off.off_reason = Some(OffReason::TimeMissed);
    off.next_due = None;
    off.last_handled_due = None;
    off.last_missed = None;
    let schedules = [
        schedule(
            1,
            "Once",
            Repeat::Once {
                date: date(2026, 6, 10),
                time: time(9, 30, 0, 0),
            },
        ),
        schedule(
            2,
            "Daily",
            Repeat::Daily {
                time: time(8, 0, 0, 0),
            },
        ),
        schedule(
            3,
            "Weekly",
            Repeat::Weekly {
                days: vec![Weekday::Monday, Weekday::Sunday],
                time: time(23, 59, 0, 0),
            },
        ),
        hourly(4, "Hours"),
        off,
    ];
    let first_run;
    {
        let repository = repository_at(&path);
        for schedule in &schedules {
            repository.save(schedule, cap(50)).expect("save");
        }
        first_run = repository
            .record_run(&run(1, 10, Some(outcome(RunResult::Missed))))
            .expect("run");
    }

    let repository = repository_at(&path);
    let mut expected = schedules.to_vec();
    expected.sort_by_key(|schedule| schedule.name.to_lowercase());
    assert_eq!(repository.list().expect("list"), expected);
    assert_eq!(
        repository.get(schedules[3].id).expect("get"),
        Some(schedules[3].clone())
    );
    let kept = repository.get_run(first_run).expect("run").expect("row");
    assert_eq!(kept.outcome, Some(outcome(RunResult::Missed)));
}

#[test]
fn a_save_with_an_existing_id_replaces_the_row() {
    let repository = memory_repository();
    repository
        .save(&hourly(1, "Hours"), cap(50))
        .expect("insert");
    let mut changed = hourly(1, "Hours");
    changed.text = "new text".to_owned();
    changed.switched_on = false;
    repository
        .save(&changed, cap(1))
        .expect("replace at the cap");
    assert_eq!(repository.list().expect("list"), [changed]);
}

#[test]
fn a_name_is_unique_without_case() {
    let repository = memory_repository();
    repository
        .save(&hourly(1, "Nightly"), cap(50))
        .expect("first");
    assert_eq!(
        repository.save(&hourly(2, "NIGHTLY"), cap(50)),
        Err(ScheduleApplicationError::NameTaken {
            name: "NIGHTLY".to_owned()
        })
    );
    repository
        .save(&hourly(1, "nightly"), cap(50))
        .expect("own name in other case");
    assert_eq!(repository.list().expect("list").len(), 1);
}

#[test]
fn the_cap_refuses_an_insert_past_the_cap() {
    let repository = memory_repository();
    repository.save(&hourly(1, "One"), cap(2)).expect("one");
    repository.save(&hourly(2, "Two"), cap(2)).expect("two");
    assert_eq!(
        repository.save(&hourly(3, "Three"), cap(2)),
        Err(ScheduleApplicationError::CapReached { cap: 2 })
    );
    assert_eq!(repository.list().expect("list").len(), 2);
}

#[test]
fn two_saves_at_the_cap_keep_exactly_one() {
    let folder = tempfile::tempdir().expect("folder");
    let path = folder.path().join("office.db");
    drop(open(&path));
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (1..=2_u128)
        .map(|number| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let repository = repository_at(&path);
                barrier.wait();
                repository.save(&hourly(number, &format!("Schedule {number}")), cap(1))
            })
        })
        .collect();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("thread"))
        .collect();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(results.contains(&Err(ScheduleApplicationError::CapReached { cap: 1 })));
    assert_eq!(repository_at(&path).list().expect("list").len(), 1);
}

#[test]
fn the_history_keeps_the_newest_twenty_runs_of_each_schedule() {
    let repository = memory_repository();
    repository.save(&hourly(1, "One"), cap(50)).expect("one");
    repository.save(&hourly(2, "Two"), cap(50)).expect("two");
    repository
        .record_run(&run(2, 0, Some(outcome(RunResult::Submitted))))
        .expect("other");
    for due in 1..=25 {
        repository
            .record_run(&run(1, due, Some(outcome(RunResult::Submitted))))
            .expect("run");
    }
    let history = repository
        .history(ScheduleId::from_u128(1))
        .expect("history");
    let dues: Vec<i64> = history.iter().map(|run| run.due.as_unix_millis()).collect();
    assert_eq!(dues, (6..=25).rev().collect::<Vec<_>>());
    assert_eq!(
        repository
            .history(ScheduleId::from_u128(2))
            .expect("other")
            .len(),
        1
    );
    let rows: i64 = repository
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM schedule_runs WHERE schedule_id = ?1",
            params![ScheduleId::from_u128(1).to_string()],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(rows, 20);
}

#[test]
fn an_open_run_settles_and_a_waiting_run_settles_again() {
    let repository = memory_repository();
    repository.save(&hourly(1, "One"), cap(50)).expect("one");
    let id = repository.record_run(&run(1, 1, None)).expect("open");
    assert_eq!(
        repository.get_run(id).expect("get").expect("row").outcome,
        None
    );
    let waiting = RunOutcome {
        result: RunResult::Waiting,
        reason: None,
        queue_entry: Some("entry-7".to_owned()),
    };
    assert_eq!(repository.settle_run(id, &waiting), Ok(true));
    assert_eq!(
        repository.get_run(id).expect("get").expect("row").outcome,
        Some(waiting)
    );
    assert_eq!(
        repository.settle_run(RunId(999), &outcome(RunResult::Failed)),
        Ok(false)
    );
}

#[test]
fn open_and_waiting_runs_are_failed_at_a_start_and_settled_ones_stay() {
    let repository = memory_repository();
    repository.save(&hourly(1, "One"), cap(50)).expect("one");
    let open = repository.record_run(&run(1, 1, None)).expect("open");
    let done = repository
        .record_run(&run(1, 2, Some(outcome(RunResult::Submitted))))
        .expect("done");
    let waiting = repository
        .record_run(&run(1, 3, Some(outcome(RunResult::Waiting))))
        .expect("waiting");

    assert_eq!(repository.fail_open_runs("the server stopped"), Ok(2));

    let failed = repository
        .get_run(open)
        .expect("get")
        .expect("row")
        .outcome
        .expect("outcome");
    assert_eq!(failed.result, RunResult::Failed);
    assert_eq!(failed.reason.as_deref(), Some("the server stopped"));
    assert_eq!(
        repository.get_run(done).expect("get").expect("row").outcome,
        Some(outcome(RunResult::Submitted))
    );
    let failed_waiting = repository
        .get_run(waiting)
        .expect("get")
        .expect("row")
        .outcome
        .expect("outcome");
    assert_eq!(failed_waiting.result, RunResult::Failed);
    assert_eq!(failed_waiting.reason.as_deref(), Some("the server stopped"));
}

#[test]
fn the_history_trim_keeps_open_and_waiting_rows() {
    let repository = memory_repository();
    repository.save(&hourly(1, "One"), cap(50)).expect("one");
    let open = repository.record_run(&run(1, 1, None)).expect("open");
    let waiting = repository
        .record_run(&run(1, 2, Some(outcome(RunResult::Waiting))))
        .expect("waiting");
    for due in 3..=30 {
        repository
            .record_run(&run(1, due, Some(outcome(RunResult::Submitted))))
            .expect("run");
    }
    for id in [open, waiting] {
        assert!(repository.get_run(id).expect("get").is_some());
    }
    let rows: i64 = repository
        .connection()
        .query_row("SELECT COUNT(*) FROM schedule_runs", [], |row| row.get(0))
        .expect("count");
    assert_eq!(rows, 22);
}

#[test]
fn a_settled_run_takes_no_late_result() {
    let repository = memory_repository();
    repository.save(&hourly(1, "One"), cap(50)).expect("one");
    let id = repository.record_run(&run(1, 1, None)).expect("open");
    assert_eq!(
        repository.settle_run(id, &outcome(RunResult::Submitted)),
        Ok(true)
    );
    assert_eq!(
        repository.settle_run(id, &outcome(RunResult::Waiting)),
        Ok(false)
    );
    assert_eq!(
        repository.get_run(id).expect("get").expect("row").outcome,
        Some(outcome(RunResult::Submitted))
    );
}

#[test]
fn delete_removes_the_schedule_and_its_history() {
    let repository = memory_repository();
    repository.save(&hourly(1, "One"), cap(50)).expect("one");
    repository.record_run(&run(1, 1, None)).expect("run");
    assert_eq!(repository.delete(ScheduleId::from_u128(1)), Ok(true));
    assert_eq!(repository.delete(ScheduleId::from_u128(1)), Ok(false));
    assert_eq!(repository.list().expect("list"), []);
    assert_eq!(repository.history(ScheduleId::from_u128(1)), Ok(vec![]));
}

#[test]
fn a_damaged_row_is_a_storage_failure_not_a_schedule() {
    let repository = memory_repository();
    repository.save(&hourly(1, "One"), cap(50)).expect("one");
    repository
        .connection()
        .execute("UPDATE schedule_schedules SET repeat_hours = NULL", [])
        .expect("damage");
    assert!(matches!(
        repository.list(),
        Err(ScheduleApplicationError::Storage { .. })
    ));
}
