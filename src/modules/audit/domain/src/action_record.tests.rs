// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{ActionKind, ActionResult};

#[test]
fn every_action_name_parses_back_to_its_action() {
    for kind in ActionKind::ALL {
        assert_eq!(kind.as_str().parse::<ActionKind>(), Ok(kind));
    }
}

#[test]
fn an_unknown_action_name_is_named_in_the_error() {
    let error = "explode".parse::<ActionKind>().expect_err("unknown");

    assert_eq!(error.to_string(), "unknown action: explode");
}

#[test]
fn a_result_round_trips_through_its_text() {
    assert_eq!(
        ActionResult::from_text(ActionResult::Ok.as_text()),
        ActionResult::Ok
    );
    let failed = ActionResult::Failed("no such folder".to_owned());
    assert_eq!(ActionResult::from_text(failed.as_text()), failed);
}

#[test]
fn hold_and_release_have_their_wire_names() {
    assert_eq!(ActionKind::Hold.as_str(), "hold");
    assert_eq!(ActionKind::Release.as_str(), "release");
    assert_eq!("hold".parse::<ActionKind>(), Ok(ActionKind::Hold));
    assert_eq!("release".parse::<ActionKind>(), Ok(ActionKind::Release));
    assert_eq!(ActionKind::ALL.len(), 13);
}

#[test]
fn the_task_text_kind_has_its_wire_name() {
    assert_eq!(ActionKind::TaskText.as_str(), "task_text");
    assert_eq!("task_text".parse::<ActionKind>(), Ok(ActionKind::TaskText));
}
