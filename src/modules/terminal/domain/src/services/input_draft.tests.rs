// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn after(inputs: &[&[u8]]) -> InputDraft {
    let mut draft = InputDraft::default();
    for input in inputs {
        draft.feed(input);
    }
    draft
}

#[test]
fn typed_text_is_unsent_until_enter() {
    assert!(after(&[b"half a line"]).is_unsent());
    assert!(!after(&[b"half a line", b"\r"]).is_unsent());
    assert!(!InputDraft::default().is_unsent());
}

#[test]
fn erasing_every_character_clears_the_line() {
    assert!(!after(&[b"ab", b"\x7f\x7f"]).is_unsent());
    assert!(after(&[b"abc", b"\x7f\x08"]).is_unsent());
    assert!(!after(&[b"\x7f\x7f"]).is_unsent());
}

#[test]
fn a_line_clearing_key_clears_the_line() {
    assert!(!after(&[b"draft", b"\x15"]).is_unsent());
    assert!(!after(&[b"draft", b"\x03"]).is_unsent());
}

#[test]
fn an_arrow_or_a_word_erase_keeps_the_line_held() {
    // Up may recall an earlier line; the count cannot see what it holds.
    assert!(after(&[b"\x1b[A"]).is_unsent());
    assert!(after(&[b"\x1bOA"]).is_unsent());
    assert!(after(&[b"\x17"]).is_unsent());
    assert!(after(&[b"\x1b"]).is_unsent());
    assert!(!after(&[b"\x1b[A", b"\r"]).is_unsent());
}

#[test]
fn a_line_break_inside_a_paste_does_not_send_it() {
    assert!(after(&[b"\x1b[200~one\rtwo\x1b[201~"]).is_unsent());
    assert!(!after(&[b"\x1b[200~one\x1b[201~", b"\r"]).is_unsent());
}

#[test]
fn focus_and_mouse_reports_edit_nothing() {
    assert!(!after(&[b"\x1b[I", b"\x1b[O", b"\x1b[<0;10;5M", b"\x1b[<0;10;5m"]).is_unsent());
}

#[test]
fn an_escape_sequence_split_across_inputs_is_read_once() {
    assert!(!after(&[b"\x1b[", b"I"]).is_unsent());
    assert!(!after(&[b"\x1b", b"[200~x\x1b[2", b"01~\x7f"]).is_unsent());
}
