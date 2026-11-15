// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn queries(scanner: &mut QueryScanner, bytes: &[u8]) -> Vec<TerminalQuery> {
    scanner
        .scan(bytes)
        .into_iter()
        .map(|found| found.query)
        .collect()
}

#[test]
fn finds_each_supported_query() {
    let mut scanner = QueryScanner::default();
    let found = queries(&mut scanner, b"a\x1b[6nb\x1b[c\x1b[0c\x1b[>c\x1b[5n");
    assert_eq!(
        found,
        [
            TerminalQuery::CursorPosition,
            TerminalQuery::DeviceAttributes,
            TerminalQuery::DeviceAttributes,
            TerminalQuery::SecondaryDeviceAttributes,
            TerminalQuery::DeviceStatus,
        ]
    );
}

#[test]
fn a_query_split_across_two_reads_is_found_once() {
    let mut scanner = QueryScanner::default();
    assert_eq!(queries(&mut scanner, b"text\x1b["), Vec::new());
    assert_eq!(queries(&mut scanner, b"6"), Vec::new());
    let found = scanner.scan(b"nmore");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].end, 1);
    assert_eq!(found[0].query, TerminalQuery::CursorPosition);
}

#[test]
fn ignores_other_sequences() {
    let mut scanner = QueryScanner::default();
    assert_eq!(
        queries(&mut scanner, b"\x1b[31m\x1b[2J\x1b[?1049h\x1b[12;40H"),
        Vec::new()
    );
}

#[test]
fn an_escape_inside_a_sequence_starts_over() {
    let mut scanner = QueryScanner::default();
    assert_eq!(
        queries(&mut scanner, b"\x1b[1\x1b[6n"),
        [TerminalQuery::CursorPosition]
    );
}

#[test]
fn long_parameters_are_not_a_query() {
    let mut scanner = QueryScanner::default();
    assert_eq!(queries(&mut scanner, b"\x1b[000000006n"), Vec::new());
}
