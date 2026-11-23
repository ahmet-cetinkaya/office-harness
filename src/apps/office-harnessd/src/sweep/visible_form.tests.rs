// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const HASH: [u8; 32] = [0xab; 32];

#[test]
fn a_plain_line_is_shown_as_it_is() {
    assert_eq!(
        visible_command("sleep 600 --flag", &HASH),
        "sleep 600 --flag"
    );
}

#[test]
fn control_escape_and_bidirectional_characters_are_written_as_escapes() {
    let line = "a\u{1b}[31m\nb\u{9b}c\u{202e}d\u{2066}e\u{200f}f\u{61c}g\u{7f}";

    let shown = visible_command(line, &HASH);

    assert_eq!(
        shown,
        "a\\u{1b}[31m\\u{a}b\\u{9b}c\\u{202e}d\\u{2066}e\\u{200f}f\\u{61c}g\\u{7f}"
    );
    assert!(shown.chars().all(|character| !is_unsafe(character)));
}

#[test]
fn a_line_at_the_limit_gets_an_ellipsis_and_the_hash_start() {
    let line = "x".repeat(COMMAND_DISPLAY_LIMIT);

    let shown = visible_command(&line, &HASH);

    assert_eq!(shown, format!("{line}…abababab"));
}

#[test]
fn a_line_below_the_limit_has_no_ellipsis() {
    let line = "x".repeat(COMMAND_DISPLAY_LIMIT - 1);

    assert_eq!(visible_command(&line, &HASH), line);
}

#[test]
fn the_hash_start_is_eight_lowercase_hex_characters() {
    let mut hash = [0_u8; 32];
    hash[..4].copy_from_slice(&[0x01, 0xa2, 0x0f, 0xff]);
    let line = "y".repeat(COMMAND_DISPLAY_LIMIT);

    assert!(visible_command(&line, &hash).ends_with("…01a20fff"));
}

#[test]
fn text_that_is_not_a_command_is_escaped_too() {
    assert_eq!(escape_unsafe("denied\u{0}"), "denied\\u{0}");
}
