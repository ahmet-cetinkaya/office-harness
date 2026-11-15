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
