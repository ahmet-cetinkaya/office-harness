// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const STAT_TAIL: &str =
    "S 1 7 7 0 -1 4194560 100 0 0 0 1 2 0 0 20 0 1 0 987654 1000 100 18446744073709551615";

#[test]
fn reads_the_state_and_the_start_ticks() {
    let fields = parse_stat(&format!("42 (node) {STAT_TAIL}")).expect("a valid line");
    assert_eq!(
        fields,
        StatFields {
            state: "S".to_owned(),
            started_at: 987_654
        }
    );
}

#[test]
fn a_command_name_with_spaces_and_parentheses_does_not_shift_the_fields() {
    let fields = parse_stat(&format!("42 (a) b (c)) {STAT_TAIL}")).expect("a valid line");
    assert_eq!(fields.started_at, 987_654);
}

#[test]
fn a_short_or_broken_line_is_not_readable() {
    assert_eq!(parse_stat("42 (node) S 1 7"), None);
    assert_eq!(parse_stat("no parenthesis here"), None);
}

#[test]
fn a_process_that_is_not_there_is_not_running() {
    assert_eq!(read_identity(u32::MAX), Err(ReadError::NotRunning));
}

#[test]
fn reads_this_process() {
    let identity = read_identity(std::process::id()).expect("this process is readable");
    assert!(identity.started_at > 0);
    assert_ne!(identity.boot, "");
}
