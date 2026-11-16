// SPDX-License-Identifier: AGPL-3.0-or-later

use terminal_domain::services::QueryScanner;
use terminal_domain::value_objects::TerminalSize;

const LINE_BREAK: &str = "\r\n";
const ENTER_ALTERNATE_SCREEN: &str = "\x1b[?1049h";

/// The server-side copy of one terminal screen. It also answers the terminal
/// queries that the program asks, so the answer does not depend on a client.
pub struct ScreenEmulator {
    parser: vt100::Parser,
    scanner: QueryScanner,
    size: TerminalSize,
}

impl ScreenEmulator {
    #[must_use]
    pub fn new(size: TerminalSize, scrollback_lines: usize) -> Self {
        Self {
            parser: vt100::Parser::new(size.rows(), size.columns(), scrollback_lines),
            scanner: QueryScanner::default(),
            size,
        }
    }

    /// Feeds output into the screen and returns the bytes to write back to
    /// the program: one answer for each query. A query split across reads is
    /// answered once, when its last byte arrives. The cursor in a cursor
    /// answer is where the cursor stands right after the query.
    pub fn process(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut replies = Vec::new();
        let mut processed = 0;
        for found in self.scanner.scan(bytes) {
            self.parser.process(&bytes[processed..found.end]);
            processed = found.end;
            let (row, column) = self.parser.screen().cursor_position();
            replies.extend(found.query.reply(row, column));
        }
        self.parser.process(&bytes[processed..]);
        replies
    }

    pub fn resize(&mut self, size: TerminalSize) {
        self.parser.set_size(size.rows(), size.columns());
        self.size = size;
    }

    #[must_use]
    pub fn bracketed_paste(&self) -> bool {
        self.parser.screen().bracketed_paste()
    }

    /// The text of the row where the cursor stands.
    #[must_use]
    pub fn cursor_line(&self) -> String {
        let screen = self.parser.screen();
        let (row, _) = screen.cursor_position();
        let columns = screen.size().1.get();
        screen.contents_between(row, 0, row, columns)
    }

    /// Terminal bytes that redraw this screen on an empty terminal: the
    /// scrollback rows, the switch to the alternate screen when it is in use,
    /// then the screen and the input modes (cursor keys, bracketed paste,
    /// mouse). `state_formatted` already ends with the input modes, but it
    /// never switches screens, so the switch is written here.
    pub fn snapshot(&mut self) -> Vec<u8> {
        let mut text = self.scrollback_text();
        let screen = self.parser.screen();
        if screen.alternate_screen() {
            text.push_str(ENTER_ALTERNATE_SCREEN);
        }
        text.push_str(&screen.state_formatted());
        text.into_bytes()
    }

    /// The rows above the screen, oldest first, each ended by a line break so
    /// the client's own scrollback receives them.
    fn scrollback_text(&mut self) -> String {
        let width = self.size.columns().get();
        let page = usize::from(self.size.rows().get());
        let screen = self.parser.screen_mut();
        let mut remaining = screen.scrollback_len();
        let mut text = String::new();
        while remaining > 0 {
            // With the view scrolled up by `remaining` rows, the first rows
            // of the view are the oldest rows not yet collected.
            screen.set_scrollback(remaining);
            let take = remaining.min(page);
            for row in screen.rows_formatted(0, width).take(take) {
                text.push_str(&row);
                text.push_str(LINE_BREAK);
            }
            remaining -= take;
        }
        screen.set_scrollback(0);
        text
    }
}

#[cfg(test)]
#[path = "screen_emulator.tests.rs"]
mod tests;
