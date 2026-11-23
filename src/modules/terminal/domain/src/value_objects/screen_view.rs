// SPDX-License-Identifier: AGPL-3.0-or-later

/// The visible rows of a terminal screen as plain text, and the row and
/// column where the cursor stands. The parser has already applied every
/// redraw, so no color or cursor code reaches these rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScreenView {
    pub rows: Vec<String>,
    pub cursor_row: usize,
    pub cursor_column: usize,
}

impl ScreenView {
    /// The cursor row and up to `above` rows over it, top first: the rows a
    /// line that wraps, or a framed input box, fills.
    #[must_use]
    pub fn rows_at_cursor(&self, above: usize) -> &[String] {
        if self.rows.is_empty() {
            return &[];
        }
        let last = self.cursor_row.min(self.rows.len() - 1);
        &self.rows[last.saturating_sub(above)..=last]
    }

    /// The last `count` rows that show anything, top first.
    #[must_use]
    pub fn bottom_rows(&self, count: usize) -> Vec<&str> {
        let mut shown: Vec<&str> = self
            .rows
            .iter()
            .rev()
            .map(String::as_str)
            .filter(|row| !row.trim().is_empty())
            .take(count)
            .collect();
        shown.reverse();
        shown
    }
}

#[cfg(test)]
#[path = "screen_view.tests.rs"]
mod tests;
