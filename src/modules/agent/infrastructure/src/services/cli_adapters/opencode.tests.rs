// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use super::*;

const SESSION_UUID: &str = "c54938eb-c04e-4ff7-bd9c-06f1c9375f91";
/// 2026-11-27 11:12:17 UTC. A real id from that second, `ses_de34b53aeffe...`,
/// was made by the CLI itself.
const NOW_MILLIS: u64 = 1_795_777_937_000;

#[test]
fn an_id_has_the_prefix_twelve_counter_digits_and_fourteen_alphanumerics() {
    let id = opencode_session_id(SESSION_UUID, NOW_MILLIS);

    let rest = id.strip_prefix("ses_").expect("starts with ses_");
    assert_eq!(rest.len(), 26);
    assert!(rest.chars().all(|digit| digit.is_ascii_alphanumeric()));
    assert!(rest[..12].chars().all(|digit| digit.is_ascii_hexdigit()));
}

#[test]
fn the_counter_digits_match_the_ones_the_cli_made_in_the_same_second() {
    let id = opencode_session_id(SESSION_UUID, NOW_MILLIS);

    // The CLI's own id of that second began `de34b5`; the slot number and the
    // sub-second part only change the digits after it.
    assert!(id.starts_with("ses_de34b"), "{id}");
}

#[test]
fn the_same_session_id_and_clock_reading_give_the_same_id() {
    assert_eq!(
        opencode_session_id(SESSION_UUID, NOW_MILLIS),
        opencode_session_id(SESSION_UUID, NOW_MILLIS)
    );
}

#[test]
fn two_sessions_started_in_the_same_millisecond_get_different_ids() {
    let other = "0a7d9c21-1b3e-4f60-8a52-3d94e7c0b6f8";

    assert_ne!(
        opencode_session_id(SESSION_UUID, NOW_MILLIS),
        opencode_session_id(other, NOW_MILLIS)
    );
}

#[test]
fn a_later_start_gives_a_smaller_counter() {
    let first = opencode_session_id(SESSION_UUID, NOW_MILLIS);
    let later = opencode_session_id(SESSION_UUID, NOW_MILLIS + 5_000);

    assert!(later[4..16] < first[4..16], "{first} {later}");
}

#[test]
fn a_short_or_odd_proposed_id_still_gives_an_id_of_the_same_shape() {
    for proposed in ["", "x", "abc"] {
        let id = opencode_session_id(proposed, NOW_MILLIS);
        assert_eq!(id.len(), 4 + 26, "{proposed:?}");
    }
}

#[test]
fn a_continue_passes_a_stored_id_of_the_older_shape_unchanged() {
    let plan = SessionPlan {
        fixed_arguments: &[],
        flags: &[],
        startup_prompt: None,
        proposed_cli_session_id: SESSION_UUID,
    };
    let older_id = "ses_c54938ebc04e4ff7bd9c06f1c9375f91";

    let command = OpencodeAdapter
        .continue_session(&plan, older_id)
        .expect("opencode continues");

    assert_eq!(command.arguments, ["--session", older_id]);
}
