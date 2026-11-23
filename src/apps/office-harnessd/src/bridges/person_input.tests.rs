// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const RUN: RunId = RunId(4);
const NEWER_RUN: RunId = RunId(5);

/// A person's keys that leave nothing unsent: one letter, then Enter.
const SENT: &[u8] = b"y\r";

fn agent() -> AgentId {
    AgentId::from_u128(9)
}

#[test]
fn a_terminal_nobody_typed_in_holds_nothing() {
    assert!(!PersonInput::default().holds_directives(agent(), Some(RUN)));
}

#[test]
fn a_person_holds_directives_for_the_typing_pause() {
    let input = PersonInput::default();
    let typed = Instant::now();
    input.stamp_at(agent(), SENT, typed);
    let almost = PERSON_TYPING_PAUSE
        .checked_sub(Duration::from_millis(1))
        .expect("the pause is longer than 1 ms");
    assert!(input.holds_directives_at(agent(), Some(RUN), typed + almost));
    assert!(!input.holds_directives_at(agent(), Some(RUN), typed + PERSON_TYPING_PAUSE));
}

#[test]
fn office_text_holds_directives_until_the_person_types() {
    let input = PersonInput::default();
    input.office_text_left(agent(), RUN);
    let later = Instant::now() + PERSON_TYPING_PAUSE * 10;
    assert!(input.holds_directives_at(agent(), Some(RUN), later));

    let typed = Instant::now();
    input.stamp_at(agent(), SENT, typed);
    assert!(!input.holds_directives_at(agent(), Some(RUN), typed + PERSON_TYPING_PAUSE));
}

#[test]
fn an_unsent_line_holds_directives_past_the_pause_until_it_is_sent_or_cleared() {
    let input = PersonInput::default();
    let typed = Instant::now();
    input.stamp_at(agent(), b"half a line", typed);
    let much_later = typed + PERSON_TYPING_PAUSE * 10;
    assert!(input.holds_directives_at(agent(), Some(RUN), much_later));

    input.stamp_at(agent(), b"\x15", typed);
    assert!(!input.holds_directives_at(agent(), Some(RUN), much_later));
}

#[test]
fn a_new_session_starts_with_nothing_held() {
    let input = PersonInput::default();
    input.stamp(agent(), b"half a line");
    input.office_text_left(agent(), RUN);
    input.session_started(agent());
    assert!(!input.holds_directives(agent(), Some(RUN)));
}

#[test]
fn office_text_of_an_older_run_holds_nothing() {
    let input = PersonInput::default();
    input.office_text_left(agent(), RUN);
    assert!(!input.holds_directives(agent(), Some(NEWER_RUN)));
    assert!(!input.holds_directives(agent(), None));
}

#[test]
fn clones_share_one_record() {
    let input = PersonInput::default();
    input.clone().office_text_left(agent(), RUN);
    assert!(input.holds_directives(agent(), Some(RUN)));
}

#[test]
fn sent_keys_hold_no_text_but_unsent_keys_and_office_text_do() {
    let input = PersonInput::default();
    input.stamp(agent(), SENT);
    assert!(!input.holds_text(agent(), Some(RUN)));

    input.stamp(agent(), b"half a line");
    assert!(input.holds_text(agent(), Some(RUN)));

    let other = AgentId::from_u128(10);
    input.office_text_left(other, RUN);
    assert!(input.holds_text(other, Some(RUN)));
    assert!(!input.holds_text(other, Some(NEWER_RUN)));
}

#[test]
fn the_time_since_the_last_input_is_known_only_after_an_input() {
    let input = PersonInput::default();
    assert_eq!(input.since_last_input(agent()), None);
    input.stamp(agent(), SENT);
    let since = input
        .since_last_input(agent())
        .expect("an input was stamped");
    assert!(since < PERSON_TYPING_PAUSE);
}
