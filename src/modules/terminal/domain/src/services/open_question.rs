// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::value_objects::ScreenView;

/// How many of the bottom rows that show anything are read: a question
/// dialog sits at the bottom, and older output above it must not hold a
/// directive back.
const QUESTION_ROWS: usize = 15;

/// Answers that a single key gives, as a CLI prints them.
const YES_NO_CHOICES: [&str; 4] = ["(y/n)", "[y/n]", "(yes/no)", "[yes/no]"];

/// Words of a question that asks the person to allow or approve something.
const PERMISSION_PHRASES: [&str; 10] = [
    "do you want to",
    "do you trust",
    "allow once",
    "allow always",
    "allow this",
    "approve this",
    "permission required",
    "requires permission",
    "press enter to confirm",
    "press enter to continue",
];

/// The marks a CLI puts before the selected entry of a choice list.
const SELECTION_MARKS: [char; 7] = ['❯', '›', '▶', '►', '→', '●', '>'];

/// True when the bottom of the screen shows a question that a key answers:
/// a yes-or-no choice, a numbered choice list with a selection mark, or
/// words that ask for permission or approval (0012/FR-002). A false match
/// only delays a directive, so the markers lean toward matching.
#[must_use]
pub fn shows_open_question(screen: &ScreenView) -> bool {
    screen
        .bottom_rows(QUESTION_ROWS)
        .into_iter()
        .any(row_asks_a_question)
}

fn row_asks_a_question(row: &str) -> bool {
    let lowered = row.to_lowercase();
    YES_NO_CHOICES.iter().any(|choice| lowered.contains(choice))
        || PERMISSION_PHRASES
            .iter()
            .any(|phrase| lowered.contains(phrase))
        || starts_a_selected_choice(row)
}

/// A selection mark, then a number with `.` or `)`, as in `❯ 1. Yes`.
fn starts_a_selected_choice(row: &str) -> bool {
    let Some(rest) = row.trim_start().strip_prefix(SELECTION_MARKS) else {
        return false;
    };
    let rest = rest.trim_start();
    let after_digits = rest.trim_start_matches(|character: char| character.is_ascii_digit());
    after_digits.len() < rest.len() && after_digits.starts_with(['.', ')'])
}

#[cfg(test)]
#[path = "open_question.tests.rs"]
mod tests;
