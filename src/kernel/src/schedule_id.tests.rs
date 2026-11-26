// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn round_trips_through_text() {
    let schedule_id = ScheduleId::generate();
    assert_eq!(
        schedule_id.to_string().parse::<ScheduleId>(),
        Ok(schedule_id)
    );
}

#[test]
fn rejects_text_that_is_not_a_uuid() {
    assert_eq!(
        "not-an-id".parse::<ScheduleId>(),
        Err(ScheduleIdError("not-an-id".to_owned()))
    );
}
