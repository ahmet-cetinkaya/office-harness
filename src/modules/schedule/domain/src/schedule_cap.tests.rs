// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn bounds_are_one_to_two_hundred() {
    for value in [1, 50, 200] {
        assert_eq!(
            ScheduleCap::new(value).expect("in range").get(),
            u32::try_from(value).expect("small")
        );
    }
    for value in [0, -1, 201, i64::MAX, i64::MIN] {
        assert_eq!(ScheduleCap::new(value), Err(ScheduleCapError));
    }
}

#[test]
fn the_default_is_fifty() {
    assert_eq!(ScheduleCap::default().get(), 50);
}

#[test]
fn the_refusal_names_the_range() {
    assert_eq!(
        ScheduleCapError.to_string(),
        "the schedule cap must be a whole number from 1 to 200"
    );
}
