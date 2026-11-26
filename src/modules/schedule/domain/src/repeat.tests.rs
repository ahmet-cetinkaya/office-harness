// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn every_weekday_code_round_trips() {
    for code in ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] {
        let day = parse_weekday(code).expect("code");
        assert_eq!(weekday_code(day), code);
    }
    assert_eq!(parse_weekday("monday"), None);
}

#[test]
fn a_time_is_two_digit_hours_and_minutes() {
    let time = parse_time("08:05").expect("time");
    assert_eq!(time_text(time), "08:05");
    for bad in [
        "8:05", "08:5", "24:00", "08:60", "0805", "", "ab:cd", "08:05:00",
    ] {
        assert_eq!(parse_time(bad), None, "{bad}");
    }
}
