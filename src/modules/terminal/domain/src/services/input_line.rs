// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::value_objects::ScreenView;

/// The rows over the cursor row that belong to the input line: a long line
/// wraps into them, and a framed input box puts its border there.
const ROWS_ABOVE_CURSOR: usize = 2;

/// How many characters at the end of a typed text must show: enough to tell
/// the text from other output, few enough to fit one wrapped row.
const TEXT_END_CHARS: usize = 24;

/// The word inside a CLI's short stand-in for a long paste, such as
/// `[Pasted text #1]`.
const PASTE_WORD: &str = "paste";

/// True when typing `text` changed the input line to show it: the end of the
/// text, or one more paste placeholder, shows there now more often than it
/// did before (0012/FR-003, A9). Spaces and frame-drawing characters are
/// left out on both sides, so a wrapped line or a framed box still matches.
#[must_use]
pub fn shows_typed_text(before: &ScreenView, after: &ScreenView, text: &str) -> bool {
    let end = text_end(text);
    if end.is_empty() {
        return false;
    }
    let line_before = input_line(before);
    let line_after = input_line(after);
    line_after.matches(&end).count() > line_before.matches(&end).count()
        || paste_placeholders(&line_after) > paste_placeholders(&line_before)
}

fn input_line(screen: &ScreenView) -> String {
    screen
        .rows_at_cursor(ROWS_ABOVE_CURSOR)
        .iter()
        .map(|row| without_spacing(row))
        .collect()
}

fn text_end(text: &str) -> String {
    let compact: Vec<char> = without_spacing(text).chars().collect();
    compact[compact.len().saturating_sub(TEXT_END_CHARS)..]
        .iter()
        .collect()
}

/// Drops whitespace and the box-drawing block (U+2500 to U+257F) that CLIs
/// draw frames with.
fn without_spacing(text: &str) -> String {
    text.chars()
        .filter(|character| {
            !character.is_whitespace() && !('\u{2500}'..='\u{257F}').contains(character)
        })
        .collect()
}

/// The number of `[...]` tokens whose text names a paste.
fn paste_placeholders(line: &str) -> usize {
    line.split('[')
        .skip(1)
        .filter_map(|after_open| after_open.split_once(']'))
        .filter(|(inside, _)| inside.to_lowercase().contains(PASTE_WORD))
        .count()
}

#[cfg(test)]
#[path = "input_line.tests.rs"]
mod tests;
