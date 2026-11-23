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

/// The fewest entries a choice list without numbers has.
const LEAST_CHOICES: usize = 2;

/// The rows that may show below a choice list without numbers, such as a
/// key hint, for the list to still end the screen.
const ROWS_BELOW_A_CHOICE_LIST: usize = 1;

/// True when the bottom of the screen shows a question that a key answers:
/// a yes-or-no choice, a choice list with a selection mark, or words that
/// ask for permission or approval (0012/FR-002). A false match only delays
/// a directive, so the markers lean toward matching.
#[must_use]
pub fn shows_open_question(screen: &ScreenView) -> bool {
    screen
        .bottom_rows(QUESTION_ROWS)
        .into_iter()
        .any(row_asks_a_question)
        || (!cursor_is_on_a_prompt(screen) && ends_with_a_choice_list(&screen.rows))
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
    let Some(rest) = text_after_selection_mark(row) else {
        return false;
    };
    let after_digits = rest.trim_start_matches(|character: char| character.is_ascii_digit());
    after_digits.len() < rest.len() && after_digits.starts_with(['.', ')'])
}

/// The row's text after a leading selection mark, as `Yes` in `❯ Yes`.
fn text_after_selection_mark(row: &str) -> Option<&str> {
    Some(row.trim_start().strip_prefix(SELECTION_MARKS)?.trim_start())
}

/// An input prompt has the terminal cursor behind a mark such as `›` or
/// `>`, in the text after it. A dialog parks the cursor on its marked entry,
/// at the mark itself or before it, as `claude` does on `❯ No, exit` of its
/// folder trust question. A list that shows beside a prompt, as a
/// `● answer` row with a `  more` row above the prompt, or a hint row under
/// it, is output or a hint, and does not hold a directive back.
fn cursor_is_on_a_prompt(screen: &ScreenView) -> bool {
    screen.rows.get(screen.cursor_row).is_some_and(|row| {
        text_after_selection_mark(row).is_some()
            && screen.cursor_column > selection_mark_column(row)
    })
}

/// The column of the row's leading selection mark: the count of the
/// characters before it. Every mark takes one column.
fn selection_mark_column(row: &str) -> usize {
    row.chars()
        .take_while(|character| character.is_whitespace())
        .count()
}

/// A choice list without numbers ends the screen, as `❯ No` over `  Yes`:
/// one entry behind a selection mark and more entries on the rows right
/// above or below it, each starting at the column where the marked entry's
/// text starts. A blank row ends the list, and only a key hint may follow
/// it, so a marked row with wrapped text in older output does not count.
fn ends_with_a_choice_list(rows: &[String]) -> bool {
    rows.iter().enumerate().any(|(index, row)| {
        selected_choice_column(row)
            .is_some_and(|column| choice_list_ends_the_screen(rows, index, column))
    })
}

/// The column where the text of a marked entry starts, as 2 in `❯ No`.
fn selected_choice_column(row: &str) -> Option<usize> {
    let text = text_after_selection_mark(row).filter(|text| !text.is_empty())?;
    Some(row.chars().count() - text.chars().count())
}

fn choice_list_ends_the_screen(rows: &[String], marked: usize, column: usize) -> bool {
    let is_entry = |row: &&String| is_choice_at_column(row, column);
    let above = rows[..marked].iter().rev().take_while(is_entry).count();
    let below = rows[marked + 1..].iter().take_while(is_entry).count();
    let rows_after = rows[marked + below + 1..]
        .iter()
        .filter(|row| !row.trim().is_empty())
        .count();
    above + 1 + below >= LEAST_CHOICES && rows_after <= ROWS_BELOW_A_CHOICE_LIST
}

/// A row with text that starts at `column` after spaces only.
fn is_choice_at_column(row: &str, column: usize) -> bool {
    let text = row.trim_start_matches(' ');
    !text.trim().is_empty() && row.len() - text.len() == column
}

#[cfg(test)]
#[path = "open_question.tests.rs"]
mod tests;
