// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn view(rows: &[&str], cursor_row: usize) -> ScreenView {
    ScreenView {
        rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        cursor_row,
    }
}

#[test]
fn the_rows_at_the_cursor_stop_at_the_top_of_the_screen() {
    let screen = view(&["a", "b", "c", "d"], 1);
    assert_eq!(screen.rows_at_cursor(2), ["a", "b"]);
    assert_eq!(
        view(&["a", "b", "c", "d"], 3).rows_at_cursor(2),
        ["b", "c", "d"]
    );
}

#[test]
fn a_cursor_below_the_rows_reads_the_last_row() {
    assert_eq!(view(&["a", "b"], 9).rows_at_cursor(0), ["b"]);
    let no_rows: &[String] = &[];
    assert_eq!(ScreenView::default().rows_at_cursor(2), no_rows);
}

#[test]
fn the_bottom_rows_skip_blank_rows() {
    let screen = view(&["one", "", "two", "  ", "three", ""], 0);
    assert_eq!(screen.bottom_rows(2), ["two", "three"]);
    assert_eq!(screen.bottom_rows(9), ["one", "two", "three"]);
}
