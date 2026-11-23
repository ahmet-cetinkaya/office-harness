// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const FIRST: &str = "first-marker-value";
const SECOND: &str = "second-marker-value-long";

fn redactor(values: &[(&str, &str)]) -> SecretRedactor {
    let redactor = SecretRedactor::new();
    redactor.replace_all(
        values
            .iter()
            .map(|(name, value)| {
                (
                    SecretName::parse(name).expect("name"),
                    SecretValue::parse((*value).to_owned()).expect("value"),
                )
            })
            .collect(),
    );
    redactor
}

fn pushed(stream: &mut dyn StreamRedactor, chunks: &[&str]) -> String {
    let mut output = Vec::new();
    for chunk in chunks {
        output.extend(stream.push(chunk.as_bytes()));
    }
    output.extend(stream.finish());
    String::from_utf8(output).expect("utf-8")
}

#[test]
fn a_value_in_text_becomes_its_placeholder_and_other_text_is_unchanged() {
    let redactor = redactor(&[("API_KEY", FIRST), ("OTHER_KEY", SECOND)]);

    assert_eq!(
        redactor.redact_text(&format!("a {FIRST} b {SECOND} c {FIRST}")),
        "a [secret API_KEY] b [secret OTHER_KEY] c [secret API_KEY]"
    );
    assert_eq!(redactor.redact_text("nothing here"), "nothing here");
    assert_eq!(redactor.redact_text(""), "");
}

#[test]
fn a_value_that_holds_another_is_replaced_whole() {
    let redactor = redactor(&[("SHORT_KEY", "abcdefgh"), ("LONG_KEY", "abcdefgh-and-more")]);

    assert_eq!(
        redactor.redact_text("x abcdefgh-and-more y"),
        "x [secret LONG_KEY] y"
    );
}

#[test]
fn a_value_split_across_two_or_three_chunks_is_still_replaced() {
    let redactor = redactor(&[("API_KEY", FIRST)]);
    let (left, right) = FIRST.split_at(7);
    let (middle_a, middle_b) = FIRST.split_at(4);

    assert_eq!(
        pushed(
            &mut *redactor.stream(),
            &[&format!("before {left}"), &format!("{right} after")]
        ),
        "before [secret API_KEY] after"
    );
    assert_eq!(
        pushed(
            &mut *redactor.stream(),
            &[
                middle_a,
                &middle_b[..4],
                &format!("{} done", &middle_b[4..])
            ]
        ),
        "[secret API_KEY] done"
    );
}

#[test]
fn the_start_of_a_value_that_never_finishes_comes_out_at_the_end() {
    let redactor = redactor(&[("API_KEY", FIRST)]);
    let mut stream = redactor.stream();

    let held = stream.push(b"text first-mark");
    assert_eq!(held, b"text ");
    assert_eq!(stream.finish(), b"first-mark");
}

#[test]
fn output_without_a_value_passes_through_and_a_change_applies_to_the_next_chunk() {
    let redactor = SecretRedactor::new();
    let mut stream = redactor.stream();
    assert_eq!(
        stream.push(b"plain \x1b[31m bytes \xff"),
        b"plain \x1b[31m bytes \xff"
    );
    assert!(redactor.knows_no_value());

    redactor.replace_all(vec![(
        SecretName::parse("API_KEY").expect("name"),
        SecretValue::parse(FIRST.to_owned()).expect("value"),
    )]);

    assert!(!redactor.knows_no_value());
    assert_eq!(
        pushed(&mut *stream, &[&format!("now {FIRST}")]),
        "now [secret API_KEY]"
    );
}

#[test]
fn a_deleted_value_is_no_longer_replaced() {
    let redactor = redactor(&[("API_KEY", FIRST)]);
    redactor.replace_all(Vec::new());

    assert_eq!(redactor.redact_text(FIRST), FIRST);
}

#[test]
fn a_cut_text_loses_the_tail_that_begins_a_value_and_keeps_the_rest() {
    let redactor = redactor(&[("API_KEY", FIRST)]);
    let cut = format!("curl -H x {}", &FIRST[..9]);

    assert_eq!(redactor.redact_cut_text(&cut), "curl -H x ");
    assert_eq!(
        redactor.redact_cut_text(&format!("curl {FIRST}")),
        "curl [secret API_KEY]"
    );
    assert_eq!(redactor.redact_cut_text("curl plain"), "curl plain");
}
