// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn a_type_within_the_limit_is_logged_whole() {
    assert_eq!(
        truncated("agent.delete", MAX_LOGGED_KIND_CHARS),
        "agent.delete"
    );
}

#[test]
fn a_type_over_the_limit_is_cut_and_marks_the_cut() {
    let long = "a".repeat(100);

    let logged = truncated(&long, MAX_LOGGED_KIND_CHARS);

    assert_eq!(logged.chars().count(), MAX_LOGGED_KIND_CHARS + 1);
    assert!(logged.ends_with('…'));
    assert_eq!(
        logged.trim_end_matches('…'),
        "a".repeat(MAX_LOGGED_KIND_CHARS)
    );
}

#[test]
fn the_limit_counts_characters_not_bytes() {
    let multibyte = "é".repeat(MAX_LOGGED_KIND_CHARS * 2);

    let logged = truncated(&multibyte, MAX_LOGGED_KIND_CHARS);

    assert_eq!(logged.chars().count(), MAX_LOGGED_KIND_CHARS + 1);
}
