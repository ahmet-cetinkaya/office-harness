// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::civil::{Weekday, date, time};

use super::*;

fn zone() -> TimeZone {
    TimeZone::get("America/New_York").expect("zone")
}

fn now() -> Timestamp {
    // 2026-06-01 12:00 UTC
    Timestamp::from_unix_millis(1_780_315_200_000)
}

fn daily() -> Repeat {
    Repeat::Daily {
        time: time(8, 0, 0, 0),
    }
}

fn check(name: &str, text: &str, repeat: &Repeat, on: bool) -> Vec<FieldError> {
    validate_fields(name, text, repeat, on, now(), &zone())
}

fn error(field: &'static str, rule: Rule) -> FieldError {
    FieldError { field, rule }
}

#[test]
fn good_fields_pass() {
    assert_eq!(check("Nightly", "ship it, çok iyi 🎉", &daily(), true), []);
}

#[test]
fn name_rules() {
    let cases = [
        ("", Rule::Blank),
        ("   ", Rule::Blank),
        (&"a".repeat(101), Rule::TooLong { max: 100 }),
        ("a\tb", Rule::ControlCharacter),
        ("a\u{1b}b", Rule::ControlCharacter),
        ("a\u{7f}b", Rule::ControlCharacter),
        ("a\u{85}b", Rule::ControlCharacter),
    ];
    for (name, rule) in cases {
        assert_eq!(
            check(name, "go", &daily(), true),
            [error("name", rule)],
            "{name:?}"
        );
    }
    assert_eq!(check(&"a".repeat(100), "go", &daily(), true), []);
}

#[test]
fn text_rules() {
    let cases = [
        ("", Rule::Blank),
        (" \n ", Rule::Blank),
        (&"a".repeat(8193), Rule::TooLong { max: 8192 }),
        ("run \u{1b}[2J", Rule::ControlCharacter),
        ("two\nlines", Rule::ControlCharacter),
        ("c1\u{9b}", Rule::ControlCharacter),
    ];
    for (text, rule) in cases {
        assert_eq!(
            check("n", text, &daily(), true),
            [error("text", rule)],
            "{text:?}"
        );
    }
    assert_eq!(check("n", &"ç".repeat(8192), &daily(), true), []);
}

#[test]
fn every_failing_field_is_named() {
    assert_eq!(
        check("", "", &Repeat::EveryHours { hours: 0 }, true),
        [
            error("name", Rule::Blank),
            error("text", Rule::Blank),
            error("repeat", Rule::HoursOutOfRange)
        ]
    );
}

#[test]
fn weekly_needs_a_day() {
    let none = Repeat::Weekly {
        days: vec![],
        time: time(8, 0, 0, 0),
    };
    assert_eq!(check("n", "t", &none, true), [error("repeat", Rule::NoDay)]);
    let one = Repeat::Weekly {
        days: vec![Weekday::Monday],
        time: time(8, 0, 0, 0),
    };
    assert_eq!(check("n", "t", &one, true), []);
}

#[test]
fn hours_are_one_to_one_hundred_sixty_eight() {
    for hours in [0, 169, u32::MAX] {
        assert_eq!(
            check("n", "t", &Repeat::EveryHours { hours }, true),
            [error("repeat", Rule::HoursOutOfRange)]
        );
    }
    for hours in [1, 24, 168] {
        assert_eq!(check("n", "t", &Repeat::EveryHours { hours }, true), []);
    }
}

#[test]
fn a_once_time_must_be_future_only_while_on() {
    let past = Repeat::Once {
        date: date(2026, 5, 31),
        time: time(8, 0, 0, 0),
    };
    let future = Repeat::Once {
        date: date(2026, 6, 2),
        time: time(8, 0, 0, 0),
    };
    assert_eq!(
        check("n", "t", &past, true),
        [error("repeat", Rule::NotInFuture)]
    );
    assert_eq!(check("n", "t", &past, false), []);
    assert_eq!(check("n", "t", &future, true), []);
}

#[test]
fn a_once_time_equal_to_now_is_not_future() {
    // 2026-06-01 08:00 New York (EDT) is 12:00 UTC.
    let at_now = Repeat::Once {
        date: date(2026, 6, 1),
        time: time(8, 0, 0, 0),
    };
    assert_eq!(
        check("n", "t", &at_now, true),
        [error("repeat", Rule::NotInFuture)]
    );
}

#[test]
fn a_name_is_trimmed_and_kept_in_normalization_form_c() {
    assert_eq!(normalize_name("  Nightly "), "Nightly");
    // "e" with a combining acute accent becomes the one letter "\u{e9}".
    assert_eq!(normalize_name("Cafe\u{301}"), "Caf\u{e9}");
    assert_eq!(normalize_name("   "), "");
}
