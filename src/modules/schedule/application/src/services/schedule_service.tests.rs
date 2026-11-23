// SPDX-License-Identifier: AGPL-3.0-or-later

use jiff::civil::{date, time};
use kernel::Timestamp;
use schedule_domain::{OffReason, Rule};

use super::*;
use crate::test_support::{HOUR, NOW, daily_at_eight, fixture, fixture_with_cap, target};

fn input(name: &str, repeat: Repeat, switched_on: bool) -> SaveInput {
    SaveInput {
        id: None,
        name: name.to_owned(),
        text: "ship the report".to_owned(),
        target: target(),
        repeat,
        switched_on,
    }
}

fn every(hours: u32) -> Repeat {
    Repeat::EveryHours { hours }
}

fn at(millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(millis)
}

#[test]
fn a_new_schedule_that_is_on_gets_its_next_due_and_handles_nothing_before_now() {
    let fixture = fixture();
    let saved = fixture
        .service
        .save(input("Nightly", daily_at_eight(), true))
        .expect("saved");
    // 2026-06-01 12:00 UTC: today's 08:00 is past, so the next is tomorrow's.
    assert_eq!(saved.next_due, Some(at(NOW + 20 * HOUR)));
    assert_eq!(saved.last_handled_due, Some(at(NOW)));
    assert_eq!(saved.anchor, at(NOW));
    assert_eq!(fixture.service.list().expect("list"), [saved]);
}

#[test]
fn a_schedule_that_is_off_has_no_next_due() {
    let saved = fixture()
        .service
        .save(input("Nightly", daily_at_eight(), false))
        .expect("saved");
    assert_eq!(saved.next_due, None);
}

#[test]
fn every_broken_field_is_named_and_nothing_is_kept() {
    let fixture = fixture();
    let mut bad = input("", every(0), true);
    bad.text = String::new();
    bad.target = AgentId::from_u128(1);
    let Err(ScheduleApplicationError::Invalid(errors)) = fixture.service.save(bad) else {
        panic!("expected invalid");
    };
    let named: Vec<_> = errors
        .iter()
        .map(|error| (error.field, error.rule))
        .collect();
    assert_eq!(
        named,
        [
            ("name", Rule::Blank),
            ("text", Rule::Blank),
            ("repeat", Rule::HoursOutOfRange),
            ("target", Rule::TargetMissing)
        ]
    );
    assert_eq!(fixture.service.list().expect("list"), []);
}

#[test]
fn a_name_is_unique_without_case_and_reported_as_a_name_rule() {
    let fixture = fixture();
    fixture
        .service
        .save(input("Nightly", daily_at_eight(), false))
        .expect("first");
    let Err(ScheduleApplicationError::Invalid(errors)) =
        fixture
            .service
            .save(input("NIGHTLY", daily_at_eight(), false))
    else {
        panic!("expected invalid");
    };
    assert_eq!((errors[0].field, errors[0].rule), ("name", Rule::NameTaken));
}

#[test]
fn the_cap_refuses_a_new_schedule_and_keeps_nothing() {
    let fixture = fixture_with_cap(1);
    fixture
        .service
        .save(input("One", daily_at_eight(), false))
        .expect("first");
    assert_eq!(
        fixture.service.save(input("Two", daily_at_eight(), false)),
        Err(ScheduleApplicationError::CapReached { cap: 1 })
    );
    assert_eq!(fixture.service.list().expect("list").len(), 1);
}

#[test]
fn an_edit_at_the_cap_is_allowed() {
    let fixture = fixture_with_cap(1);
    let first = fixture
        .service
        .save(input("One", daily_at_eight(), false))
        .expect("first");
    let mut edit = input("Renamed", daily_at_eight(), false);
    edit.id = Some(first.id);
    assert_eq!(fixture.service.save(edit).expect("edit").name, "Renamed");
}

#[test]
fn an_unknown_id_is_not_found() {
    let mut edit = input("One", daily_at_eight(), false);
    edit.id = Some(ScheduleId::from_u128(9));
    assert!(matches!(
        fixture().service.save(edit),
        Err(ScheduleApplicationError::NotFound(_))
    ));
}

#[test]
fn the_anchor_moves_only_for_a_changed_repeat_or_a_switch_on() {
    let fixture = fixture();
    let first = fixture
        .service
        .save(input("Hourly", every(3), true))
        .expect("first");
    fixture.clock.set(NOW + HOUR);

    let mut rename = input("Hourly renamed", every(3), true);
    rename.id = Some(first.id);
    assert_eq!(
        fixture.service.save(rename).expect("rename").anchor,
        at(NOW)
    );

    let mut change = input("Hourly renamed", every(4), true);
    change.id = Some(first.id);
    assert_eq!(
        fixture.service.save(change).expect("change").anchor,
        at(NOW + HOUR)
    );

    fixture.clock.set(NOW + 2 * HOUR);
    let mut off = input("Hourly renamed", every(4), false);
    off.id = Some(first.id);
    fixture.service.save(off).expect("off");
    fixture.clock.set(NOW + 3 * HOUR);
    let mut on = input("Hourly renamed", every(4), true);
    on.id = Some(first.id);
    assert_eq!(
        fixture.service.save(on).expect("on").anchor,
        at(NOW + 3 * HOUR)
    );
}

#[test]
fn a_save_clears_the_off_reason_and_marks_the_time_before_it_handled() {
    let fixture = fixture();
    let first = fixture
        .service
        .save(input("Hourly", every(1), true))
        .expect("first");
    let mut off = fixture.service.get(first.id).expect("get");
    off.switched_on = false;
    off.off_reason = Some(OffReason::TargetDeleted);
    fixture
        .store
        .save(&off, schedule_domain::ScheduleCap::default())
        .expect("store");

    fixture.clock.set(NOW + 5 * HOUR);
    let mut edit = input("Hourly", every(1), true);
    edit.id = Some(first.id);
    let saved = fixture.service.save(edit).expect("edit");
    assert_eq!(saved.off_reason, None);
    assert_eq!(saved.last_handled_due, Some(at(NOW + 5 * HOUR)));
    assert_eq!(saved.next_due, Some(at(NOW + 6 * HOUR)));
}

#[test]
fn a_once_time_in_the_past_is_refused_only_when_on() {
    let fixture = fixture();
    let past = Repeat::Once {
        date: date(2026, 5, 31),
        time: time(8, 0, 0, 0),
    };
    assert!(matches!(
        fixture.service.save(input("Once", past.clone(), true)),
        Err(ScheduleApplicationError::Invalid(_))
    ));
    assert!(fixture.service.save(input("Once", past, false)).is_ok());
}

#[test]
fn switch_off_clears_the_next_due() {
    let fixture = fixture();
    let saved = fixture
        .service
        .save(input("N", every(1), true))
        .expect("save");
    let off = fixture.service.switch(saved.id, false).expect("off");
    assert!(!off.switched_on);
    assert_eq!(off.next_due, None);
}

#[test]
fn switch_on_after_off_makes_no_run_for_the_off_time() {
    let fixture = fixture();
    let saved = fixture
        .service
        .save(input("N", every(1), true))
        .expect("save");
    fixture.service.switch(saved.id, false).expect("off");
    fixture.clock.set(NOW + 10 * HOUR + 1_000);
    let on = fixture.service.switch(saved.id, true).expect("on");
    assert_eq!(on.anchor, at(NOW + 10 * HOUR + 1_000));
    assert_eq!(on.last_handled_due, Some(at(NOW + 10 * HOUR + 1_000)));
    assert_eq!(on.next_due, Some(at(NOW + 11 * HOUR + 1_000)));
}

#[test]
fn switch_on_is_refused_for_a_once_time_that_has_passed() {
    let fixture = fixture();
    let once = Repeat::Once {
        date: date(2026, 6, 1),
        time: time(13, 0, 0, 0),
    };
    let saved = fixture
        .service
        .save(input("Once", once, true))
        .expect("save");
    fixture.service.switch(saved.id, false).expect("off");
    fixture.clock.set(NOW + 2 * HOUR);
    assert_eq!(
        fixture.service.switch(saved.id, true),
        Err(ScheduleApplicationError::SwitchRefused(
            SwitchRefusal::OnceTimePassed
        ))
    );
    assert!(!fixture.service.get(saved.id).expect("get").switched_on);
}

#[test]
fn switch_on_is_refused_for_an_off_reason_until_an_edit() {
    let fixture = fixture();
    let saved = fixture
        .service
        .save(input("N", every(1), true))
        .expect("save");
    let mut off = fixture.service.get(saved.id).expect("get");
    off.switched_on = false;
    off.off_reason = Some(OffReason::TargetDeleted);
    fixture
        .store
        .save(&off, schedule_domain::ScheduleCap::default())
        .expect("store");
    assert_eq!(
        fixture.service.switch(saved.id, true),
        Err(ScheduleApplicationError::SwitchRefused(
            SwitchRefusal::TargetDeleted
        ))
    );
    let mut edit = input("N", every(1), true);
    edit.id = Some(saved.id);
    assert!(fixture.service.save(edit).expect("edit").switched_on);
}

#[test]
fn a_switch_to_the_same_state_changes_nothing() {
    let fixture = fixture();
    let saved = fixture
        .service
        .save(input("N", every(1), true))
        .expect("save");
    assert_eq!(fixture.service.switch(saved.id, true).expect("on"), saved);
}

#[test]
fn delete_removes_the_schedule_and_says_so_when_it_is_gone() {
    let fixture = fixture();
    let saved = fixture
        .service
        .save(input("N", every(1), true))
        .expect("save");
    fixture.service.delete(saved.id).expect("delete");
    assert!(matches!(
        fixture.service.delete(saved.id),
        Err(ScheduleApplicationError::NotFound(_))
    ));
}

#[test]
fn a_name_with_spaces_at_its_ends_is_the_same_name_and_is_saved_trimmed() {
    let fixture = fixture();
    let first = fixture
        .service
        .save(input("Nightly", daily_at_eight(), false))
        .expect("first");
    let Err(ScheduleApplicationError::Invalid(errors)) =
        fixture
            .service
            .save(input("Nightly ", daily_at_eight(), false))
    else {
        panic!("expected invalid");
    };
    assert_eq!((errors[0].field, errors[0].rule), ("name", Rule::NameTaken));
    let padded = fixture
        .service
        .save(input("  Weekly  ", daily_at_eight(), false))
        .expect("padded");
    assert_eq!(first.name, "Nightly");
    assert_eq!(padded.name, "Weekly");
}
