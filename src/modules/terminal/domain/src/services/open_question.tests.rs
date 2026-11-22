// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn screen(rows: &[&str]) -> ScreenView {
    ScreenView {
        rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        cursor_row: rows.len().saturating_sub(1),
    }
}

#[test]
fn a_yes_or_no_choice_is_a_question() {
    assert!(shows_open_question(&screen(&["Overwrite the file? (y/n)"])));
    assert!(shows_open_question(&screen(&["Continue? [Y/n]"])));
    assert!(shows_open_question(&screen(&["Delete it (yes/no)?"])));
}

#[test]
fn a_choice_list_with_a_selection_mark_is_a_question() {
    let dialog = [
        "Run this command?",
        "❯ 1. Yes",
        "  2. Yes, and remember",
        "  3. No",
    ];
    assert!(shows_open_question(&screen(&dialog)));
    assert!(shows_open_question(&screen(&["› 2) Allow"])));
    assert!(shows_open_question(&screen(&["● 1. Proceed"])));
}

#[test]
fn a_request_for_permission_is_a_question() {
    assert!(shows_open_question(&screen(&["Do you want to proceed?"])));
    assert!(shows_open_question(&screen(&[
        "Do you trust the files in this folder?"
    ])));
    assert!(shows_open_question(&screen(&[
        "  Allow once   Allow always   Reject"
    ])));
    assert!(shows_open_question(&screen(&[
        "Permission required: write file"
    ])));
}

#[test]
fn an_idle_prompt_is_not_a_question() {
    assert!(!shows_open_question(&screen(&[
        "Done. The tests pass.",
        "",
        "> "
    ])));
    assert!(!shows_open_question(&screen(&["user@host:~$ "])));
    assert!(!shows_open_question(&ScreenView::default()));
}

#[test]
fn a_numbered_list_without_a_selection_mark_is_not_a_question() {
    let output = ["Next steps:", "1. Add the test", "2. Run it", "> "];
    assert!(!shows_open_question(&screen(&output)));
}

#[test]
fn a_quoted_markdown_list_row_matches_and_only_delays() {
    // A `>` quote before a numbered item reads like a selected choice. The
    // match only holds the directive back (spec A3), so it stays a match
    // rather than a guess about which `>` rows quote.
    assert!(shows_open_question(&screen(&["> 1. item", "> "])));
}

#[test]
fn a_question_above_the_bottom_rows_is_not_read() {
    let mut rows = vec!["Continue? (y/n)"];
    rows.extend(["line"; QUESTION_ROWS]);
    assert!(!shows_open_question(&screen(&rows)));
}

#[test]
fn blank_rows_do_not_push_a_question_out_of_the_bottom_rows() {
    let mut rows = vec!["Continue? (y/n)"];
    rows.extend([""; 40]);
    assert!(shows_open_question(&screen(&rows)));
}

#[test]
fn a_choice_list_without_numbers_at_the_bottom_is_a_question() {
    let dialog = [
        " Pick a mode for this folder.",
        "",
        " ❯ Leave",
        "   Stay and work here",
        "",
        " Enter to confirm · Esc to cancel",
        "",
        "",
    ];
    assert!(shows_open_question(&screen(&dialog)));
    assert!(shows_open_question(&screen(&[
        "Pick a fruit",
        "  Apple",
        "> Pear",
        "  Plum"
    ])));
}

#[test]
fn a_numbered_choice_list_with_a_hint_below_is_still_a_question() {
    let dialog = [
        " Pick a mode.",
        " ❯ 1. Leave",
        "   2. Stay",
        "",
        " Esc to cancel",
    ];
    assert!(shows_open_question(&screen(&dialog)));
}

#[test]
fn output_with_a_quote_row_is_not_a_question() {
    let output = [
        "The review says:",
        "> Keep the guard clause first.",
        "I agree with that.",
        "> ",
    ];
    assert!(!shows_open_question(&screen(&output)));
}

#[test]
fn a_marked_row_with_wrapped_text_above_more_output_is_not_a_question() {
    let output = [
        "● The answer wraps",
        "  onto a second row.",
        "",
        "────────",
        "❯ ",
        "────────",
    ];
    assert!(!shows_open_question(&screen(&output)));
}

#[test]
fn a_blank_row_ends_a_choice_list() {
    let idle = ["› Ask anything", "", "  ? for shortcuts"];
    assert!(!shows_open_question(&screen(&idle)));
}

/// A screen whose cursor sits on the row at `cursor_row`.
fn screen_with_cursor(rows: &[&str], cursor_row: usize) -> ScreenView {
    ScreenView {
        cursor_row,
        ..screen(rows)
    }
}

#[test]
fn an_idle_prompt_below_wrapped_output_is_not_a_question() {
    let idle = ["● The answer wraps", "  onto a second row.", "› "];
    assert!(!shows_open_question(&screen_with_cursor(&idle, 2)));
}

#[test]
fn an_idle_prompt_with_a_hint_at_its_text_column_is_not_a_question() {
    let idle = ["Done.", "", "> draft text", "  ? for shortcuts"];
    assert!(!shows_open_question(&screen_with_cursor(&idle, 2)));
}

#[test]
fn a_choice_list_with_the_cursor_below_it_is_still_a_question() {
    let dialog = [" ❯ Leave", "   Stay and work here", "", " Enter to confirm"];
    assert!(shows_open_question(&screen_with_cursor(&dialog, 3)));
}
