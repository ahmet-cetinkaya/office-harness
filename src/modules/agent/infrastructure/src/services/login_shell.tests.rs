// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn marked(body: &str) -> Vec<u8> {
    format!("noise before\n{MARKER}{body}{MARKER}noise after").into_bytes()
}

#[test]
fn reads_the_variables_between_the_markers() {
    let parsed = parse_marked_environment(&marked("A=1\0B=two=2\0")).expect("parsed");
    assert_eq!(parsed.get("A").map(String::as_str), Some("1"));
    assert_eq!(parsed.get("B").map(String::as_str), Some("two=2"));
}

#[test]
fn needs_both_markers() {
    let open_only = format!("{MARKER}A=1\0");
    assert!(parse_marked_environment(open_only.as_bytes()).is_none());
    assert!(parse_marked_environment(b"A=1\0").is_none());
}

#[test]
fn an_empty_answer_is_no_answer() {
    assert!(parse_marked_environment(&marked("")).is_none());
}
