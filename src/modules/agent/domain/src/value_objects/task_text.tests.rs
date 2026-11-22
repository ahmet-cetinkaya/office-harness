// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const SET_AT: Timestamp = Timestamp::from_unix_millis(7);

fn parsed(text: &str) -> Result<Option<TaskText>, AgentError> {
    TaskText::from_input(text, SET_AT, TaskSource::User)
}

#[test]
fn a_text_of_the_limit_is_accepted() {
    let task = parsed(&"x".repeat(TASK_TEXT_MAX_CHARS)).expect("valid");
    assert!(task.is_some());
}

#[test]
fn a_text_past_the_limit_is_refused_with_the_limit_in_the_message() {
    let error = parsed(&"x".repeat(TASK_TEXT_MAX_CHARS + 1)).expect_err("too long");
    assert_eq!(error, AgentError::TaskTextTooLong);
    assert_eq!(
        error.to_string(),
        "a task text has at most 8,192 characters"
    );
}

#[test]
fn the_limit_counts_characters_not_bytes() {
    let wide = "\u{1F600}".repeat(TASK_TEXT_MAX_CHARS);
    assert!(parsed(&wide).expect("valid").is_some());
}

#[test]
fn a_blank_text_clears_even_when_it_is_longer_than_the_limit() {
    assert_eq!(parsed("").expect("valid"), None);
    assert_eq!(parsed(" \t\n").expect("valid"), None);
    assert_eq!(
        parsed(&" ".repeat(TASK_TEXT_MAX_CHARS + 1)).expect("valid"),
        None
    );
}

#[test]
fn control_characters_are_kept_unchanged() {
    let text = "red \u{1b}[31m\nline two\u{7}";
    let task = parsed(text).expect("valid").expect("some");
    assert_eq!(task.text(), text);
    assert_eq!(task.set_at(), SET_AT);
    assert_eq!(task.source(), TaskSource::User);
}

#[test]
fn a_source_round_trips_through_its_text() {
    for source in [TaskSource::Directive, TaskSource::User] {
        assert_eq!(source.as_str().parse::<TaskSource>(), Ok(source));
    }
    assert!("agent".parse::<TaskSource>().is_err());
}
