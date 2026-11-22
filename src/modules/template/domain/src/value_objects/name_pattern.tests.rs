// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn taken_in<'a>(names: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
    move |candidate| {
        names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(candidate))
    }
}

#[test]
fn the_number_is_the_lowest_that_makes_the_name_unique_ignoring_case() {
    let pattern = NamePattern::new("reviewer-{n}");

    let name = pattern.unique_name(None, taken_in(&["Reviewer-1", "reviewer-3"]));

    assert_eq!(name.as_deref(), Some("reviewer-2"));
}

#[test]
fn a_pattern_without_a_number_gets_one_only_when_the_name_is_taken() {
    let pattern = NamePattern::new("scout");

    assert_eq!(
        pattern.unique_name(None, taken_in(&[])).as_deref(),
        Some("scout")
    );
    assert_eq!(
        pattern
            .unique_name(None, taken_in(&["scout", "scout-1"]))
            .as_deref(),
        Some("scout-2")
    );
}

#[test]
fn the_team_name_replaces_the_team_token_and_stays_text() {
    let pattern = NamePattern::new(MEMBER_PATTERN);

    let name = pattern.unique_name(Some("api{n}"), taken_in(&["api{n}-member-1"]));

    assert_eq!(name.as_deref(), Some("api{n}-member-2"));
}

#[test]
fn a_team_pattern_without_a_team_gives_no_name() {
    let pattern = NamePattern::new(LEAD_PATTERN);

    assert!(pattern.needs_team());
    assert_eq!(pattern.unique_name(None, taken_in(&[])), None);
    assert_eq!(
        pattern.unique_name(Some("api"), taken_in(&[])).as_deref(),
        Some("api-lead")
    );
}

#[test]
fn the_least_length_counts_literal_text_and_the_first_number() {
    assert_eq!(
        NamePattern::new("{team}-member-{n}").least_expanded_chars(),
        9
    );
    assert_eq!(NamePattern::new("é{").least_expanded_chars(), 2);
}

#[test]
fn the_default_pattern_of_an_agent_ends_with_the_number() {
    assert_eq!(default_agent_pattern("Ana"), "Ana-{n}");
}
