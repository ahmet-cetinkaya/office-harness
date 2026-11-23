// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const LONG_TEXT: &str = "Please run the full test suite and report every failure you see";

fn screen(rows: &[&str], cursor_row: usize) -> ScreenView {
    ScreenView {
        rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        cursor_row,
        cursor_column: 0,
    }
}

#[test]
fn an_echoed_text_shows() {
    let before = screen(&["ready", "> "], 1);
    let after = screen(&["ready", "> hello member"], 1);
    assert!(shows_typed_text(&before, &after, "hello member"));
}

#[test]
fn a_text_in_a_framed_input_box_shows() {
    let before = screen(&["╭──────────╮", "│ >        │", "╰──────────╯"], 1);
    let after = screen(&["╭──────────╮", "│ > hello  │", "╰──────────╯"], 1);
    assert!(shows_typed_text(&before, &after, "hello"));
}

#[test]
fn a_text_wrapped_over_two_rows_shows() {
    let before = screen(&["output", "> "], 1);
    let after = screen(
        &[
            "> Please run the full test suite and rep",
            "ort every failure you see",
        ],
        1,
    );
    assert!(shows_typed_text(&before, &after, LONG_TEXT));
}

#[test]
fn a_line_that_already_showed_the_text_does_not_count() {
    let same = screen(&["> hello member"], 0);
    assert!(!shows_typed_text(&same, &same, "hello member"));
}

#[test]
fn a_question_that_does_not_echo_shows_nothing() {
    let dialog = screen(&["Run it?", "  Yes", "  No"], 2);
    assert!(!shows_typed_text(&dialog, &dialog, "hello member"));
}

#[test]
fn a_new_paste_placeholder_shows() {
    let before = screen(&["> "], 0);
    let after = screen(&["> [Pasted text #1 +3 lines]"], 0);
    assert!(shows_typed_text(&before, &after, LONG_TEXT));
}

#[test]
fn a_placeholder_that_was_there_before_does_not_count() {
    let same = screen(&["> [Pasted text #1]"], 0);
    assert!(!shows_typed_text(&same, &same, LONG_TEXT));
}

#[test]
fn a_text_far_above_the_cursor_does_not_count() {
    let before = screen(&["", "", "", "> "], 3);
    let after = screen(&["hello member", "", "", "> "], 3);
    assert!(!shows_typed_text(&before, &after, "hello member"));
}

#[test]
fn a_blank_text_never_shows() {
    let after = screen(&["> "], 0);
    assert!(!shows_typed_text(&ScreenView::default(), &after, "   "));
}
