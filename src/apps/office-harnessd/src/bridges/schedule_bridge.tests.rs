// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::tz::TimeZone;
use kernel::{ScheduleId, Timestamp};
use schedule_application::common::models::RunResult;
use schedule_application::services::MissedBatch;

use super::{SERVER_STOPPED, SETTLED_AT_START_REASON, local_text, missed_reason, notifies};

const DAY_MILLIS: i64 = 86_400_000;

fn batch(count: u64, first: i64, last: i64) -> MissedBatch {
    MissedBatch {
        schedule_id: ScheduleId::generate(),
        count,
        first: Timestamp::from_unix_millis(first),
        last: Timestamp::from_unix_millis(last),
    }
}

#[test]
fn a_moment_reads_in_the_wall_clock_of_the_zone() {
    let berlin = TimeZone::get("Europe/Berlin").expect("zone");
    // 2023-11-14 22:13:20 UTC is 23:13 in Berlin (UTC+1).
    assert_eq!(
        local_text(Timestamp::from_unix_millis(1_700_000_000_000), &berlin),
        "2023-11-14 23:13"
    );
}

#[test]
fn a_missed_entry_names_the_count_and_the_first_and_last_time() {
    let utc = TimeZone::UTC;
    let reason = missed_reason(&batch(3, 0, 2 * DAY_MILLIS), &utc);
    assert_eq!(
        reason,
        "3 runs missed, first 1970-01-01 00:00, last 1970-01-03 00:00"
    );
    assert!(missed_reason(&batch(1, 0, 0), &utc).starts_with("1 run missed,"));
}

#[test]
fn a_result_notifies_unless_it_is_a_success_a_wait_or_a_stop_of_the_server() {
    for result in [
        RunResult::Skipped,
        RunResult::Failed,
        RunResult::Refused,
        RunResult::Unsubmitted,
    ] {
        assert!(notifies(result, Some("why")), "{}", result.as_str());
    }
    assert!(notifies(RunResult::Withdrawn, Some("10 minutes")));
    assert!(!notifies(RunResult::Withdrawn, Some(SERVER_STOPPED)));
    assert!(!notifies(RunResult::Failed, Some(SETTLED_AT_START_REASON)));
    for result in [RunResult::Submitted, RunResult::Waiting, RunResult::Missed] {
        assert!(!notifies(result, None), "{}", result.as_str());
    }
}
