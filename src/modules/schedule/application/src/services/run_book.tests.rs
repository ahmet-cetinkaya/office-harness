// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::Timestamp;

use super::*;
use crate::common::models::RunVia;
use crate::test_support::MemoryStore;

fn book() -> (RunBook, Arc<MemoryStore>) {
    let store = Arc::new(MemoryStore::default());
    (RunBook::new(store.clone()), store)
}

fn schedule_id() -> ScheduleId {
    ScheduleId::from_u128(1)
}

fn new_run(outcome: Option<RunOutcome>) -> NewRun {
    NewRun {
        schedule_id: schedule_id(),
        due: Timestamp::from_unix_millis(1),
        started: Timestamp::from_unix_millis(2),
        via: RunVia::Due,
        outcome,
    }
}

fn outcome(result: RunResult) -> RunOutcome {
    RunOutcome {
        result,
        reason: None,
        queue_entry: None,
    }
}

#[test]
fn an_open_run_blocks_the_next_one() {
    let (book, _) = book();
    assert_eq!(book.skip_reason(schedule_id()), Ok(None));
    book.open(&new_run(None)).expect("open");
    assert_eq!(book.skip_reason(schedule_id()), Ok(Some(WAITING_REASON)));
}

#[test]
fn a_waiting_run_blocks_until_it_settles() {
    let (book, _) = book();
    let id = book.open(&new_run(None)).expect("open");
    book.settle(id, &outcome(RunResult::Waiting))
        .expect("waiting");
    assert_eq!(book.skip_reason(schedule_id()), Ok(Some(WAITING_REASON)));
    book.settle(id, &outcome(RunResult::Submitted))
        .expect("submitted");
    assert_eq!(book.skip_reason(schedule_id()), Ok(None));
}

#[test]
fn waiting_settles_to_submitted_unsubmitted_or_withdrawn_only() {
    for result in [
        RunResult::Submitted,
        RunResult::Unsubmitted,
        RunResult::Withdrawn,
    ] {
        let (book, store) = book();
        let id = book
            .open(&new_run(Some(outcome(RunResult::Waiting))))
            .expect("open");
        book.settle(id, &outcome(result)).expect("settle");
        assert_eq!(
            store.get_run(id).expect("get").expect("row").outcome,
            Some(outcome(result))
        );
    }
    let (book, _) = book();
    let id = book
        .open(&new_run(Some(outcome(RunResult::Waiting))))
        .expect("open");
    assert!(matches!(
        book.settle(id, &outcome(RunResult::Failed)),
        Err(ScheduleApplicationError::RunSettled(_))
    ));
}

#[test]
fn a_settled_run_takes_nothing_more() {
    let (book, _) = book();
    let id = book
        .open(&new_run(Some(outcome(RunResult::Refused))))
        .expect("open");
    assert!(matches!(
        book.settle(id, &outcome(RunResult::Submitted)),
        Err(ScheduleApplicationError::RunSettled(_))
    ));
}

#[test]
fn an_unknown_run_is_not_found() {
    let (book, _) = book();
    assert!(matches!(
        book.settle(RunId(99), &outcome(RunResult::Submitted)),
        Err(ScheduleApplicationError::NotFound(_))
    ));
}

#[test]
fn open_runs_fail_at_a_start_with_the_server_stopped_reason() {
    let (book, store) = book();
    let open = book.open(&new_run(None)).expect("open");
    let done = book
        .open(&new_run(Some(outcome(RunResult::Submitted))))
        .expect("done");
    assert_eq!(book.fail_open_runs(), Ok(1));
    let failed = store
        .get_run(open)
        .expect("get")
        .expect("row")
        .outcome
        .expect("outcome");
    assert_eq!(failed.result, RunResult::Failed);
    assert_eq!(failed.reason.as_deref(), Some("the server stopped"));
    assert_eq!(
        store.get_run(done).expect("get").expect("row").outcome,
        Some(outcome(RunResult::Submitted))
    );
}

#[test]
fn a_run_that_waited_at_a_stop_is_failed_at_the_start_and_blocks_nothing() {
    let (book, store) = book();
    let id = book
        .open(&new_run(Some(outcome(RunResult::Waiting))))
        .expect("open");
    assert_eq!(book.skip_reason(schedule_id()), Ok(Some(WAITING_REASON)));
    assert_eq!(book.fail_open_runs(), Ok(1));
    let failed = store
        .get_run(id)
        .expect("get")
        .expect("row")
        .outcome
        .expect("outcome");
    assert_eq!(failed.result, RunResult::Failed);
    assert_eq!(book.skip_reason(schedule_id()), Ok(None));
}

#[test]
fn a_late_waiting_result_never_overwrites_a_submitted_one() {
    let (book, store) = book();
    let id = book.open(&new_run(None)).expect("open");
    book.settle(id, &outcome(RunResult::Submitted))
        .expect("submitted");
    assert!(matches!(
        book.settle(id, &outcome(RunResult::Waiting)),
        Err(ScheduleApplicationError::RunSettled(_))
    ));
    assert_eq!(
        store.get_run(id).expect("get").expect("row").outcome,
        Some(outcome(RunResult::Submitted))
    );
}
