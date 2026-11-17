// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn emulator(columns: u16, rows: u16) -> ScreenEmulator {
    ScreenEmulator::new(TerminalSize::new(columns, rows).expect("valid size"), 1000)
}

#[test]
fn answers_a_cursor_query_with_the_cursor_after_the_text_before_it() {
    let mut screen = emulator(80, 24);
    let replies = screen.process(b"abc\x1b[6ndef");
    assert_eq!(replies, b"\x1b[1;4R");
}

#[test]
fn a_query_split_across_two_reads_gets_exactly_one_answer() {
    let mut screen = emulator(80, 24);
    assert_eq!(screen.process(b"\x1b["), Vec::new());
    assert_eq!(screen.process(b"6n"), b"\x1b[1;1R");
    assert_eq!(screen.process(b"more"), Vec::new());
}

#[test]
fn the_last_line_is_the_last_non_empty_line_without_codes() {
    let mut screen = emulator(80, 24);
    screen.process(b"first\r\n\x1b[31mred plain\x1b[0m\r\n");
    assert_eq!(screen.last_line().as_deref(), Some("red plain"));
}

#[test]
fn a_carriage_return_redraw_counts_once_in_the_last_line() {
    let mut screen = emulator(80, 24);
    screen.process(b"0%\r50%\r100%\r\n");
    assert_eq!(screen.last_line().as_deref(), Some("100%"));
}

#[test]
fn a_line_wrapped_across_rows_reads_as_one_line() {
    let mut screen = emulator(10, 24);
    screen.process(b"0123456789ABCDEFGHIJ\r\n");
    assert_eq!(
        screen.last_line().as_deref(),
        Some("0123456789ABCDEFGHIJ")
    );
}

#[test]
fn an_empty_screen_has_no_last_line() {
    let mut screen = emulator(80, 24);
    assert_eq!(screen.last_line(), None);
    screen.process(b"\r\n\r\n");
    assert_eq!(screen.last_line(), None);
}

#[test]
fn the_last_line_follows_the_alternate_screen() {
    let mut screen = emulator(80, 24);
    screen.process(b"main line\r\n");
    screen.process(b"\x1b[?1049h\x1b[2J\x1b[1;1Hprogress: 40%");
    assert_eq!(screen.last_line().as_deref(), Some("progress: 40%"));
    screen.process(b"\x1b[?1049l");
    assert_eq!(screen.last_line().as_deref(), Some("main line"));
}

#[test]
fn a_bell_outside_an_osc_string_is_counted_and_taken_once() {
    let mut screen = emulator(80, 24);
    screen.process(b"ready\x07");
    assert_eq!(screen.last_line().as_deref(), Some("ready"));
    assert_eq!(screen.take_bells(), 1);
    assert_eq!(screen.take_bells(), 0);
}

#[test]
fn every_bell_of_a_repeated_ring_is_counted() {
    let mut screen = emulator(80, 24);
    screen.process(b"\x07\x07\x07");
    assert_eq!(screen.take_bells(), 3);
}

#[test]
fn a_bell_that_ends_a_window_title_is_not_a_bell() {
    let mut screen = emulator(80, 24);
    screen.process(b"\x1b]2;window title\x07after\x07");
    assert_eq!(screen.last_line().as_deref(), Some("after"));
    assert_eq!(screen.take_bells(), 1);
}

#[test]
fn a_bell_split_across_reads_is_counted_once_when_it_completes() {
    let mut screen = emulator(80, 24);
    screen.process(b"\x07");
    assert_eq!(screen.take_bells(), 1);
    screen.process(b"\x1b]2;ti");
    assert_eq!(screen.take_bells(), 0);
    screen.process(b"tle\x07");
    assert_eq!(screen.take_bells(), 0);
}

#[test]
fn a_snapshot_redraws_the_alternate_screen_and_the_input_modes() {
    let mut original = emulator(40, 10);
    original.process(b"\x1b[?1049h\x1b[2J\x1b[3;5HFULL-SCREEN\x1b[?2004h\x1b[?1h");
    let snapshot = original.snapshot();

    let mut replica = emulator(40, 10);
    replica.process(&snapshot);
    assert_eq!(
        replica.parser.screen().contents(),
        original.parser.screen().contents()
    );
    assert!(replica.parser.screen().alternate_screen());
    assert!(replica.bracketed_paste());
    assert!(replica.parser.screen().application_cursor());
}

#[test]
fn a_snapshot_carries_the_scrollback_rows() {
    let mut original = emulator(20, 4);
    for number in 0..30 {
        original.process(format!("row{number:02}\r\n").as_bytes());
    }
    let text = String::from_utf8(original.snapshot()).expect("snapshot is text");
    for number in 0..30 {
        assert_eq!(
            text.matches(&format!("row{number:02}")).count(),
            1,
            "row {number}"
        );
    }
}

#[test]
fn a_resize_changes_the_snapshot_size() {
    let mut screen = emulator(80, 24);
    screen.resize(TerminalSize::new(100, 30).expect("valid size"));
    screen.process(b"x");
    assert_eq!(screen.size, TerminalSize::new(100, 30).expect("valid size"));
}

#[test]
fn the_cursor_line_is_the_row_where_the_cursor_stands() {
    let mut screen = emulator(40, 5);
    screen.process(b"first\r\nTerminate batch job (Y/N)? ");
    assert!(
        screen
            .cursor_line()
            .starts_with("Terminate batch job (Y/N)?")
    );
}
