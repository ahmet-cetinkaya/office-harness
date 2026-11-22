// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const RUN: RunId = RunId(4);
const NEWER_RUN: RunId = RunId(5);

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
    input.stamp_at(agent(), typed);
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
    input.stamp_at(agent(), typed);
    assert!(!input.holds_directives_at(agent(), Some(RUN), typed + PERSON_TYPING_PAUSE));
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
