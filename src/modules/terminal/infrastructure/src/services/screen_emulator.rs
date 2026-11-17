// SPDX-License-Identifier: AGPL-3.0-or-later

use terminal_domain::services::QueryScanner;
use terminal_domain::value_objects::TerminalSize;

const LINE_BREAK: &str = "\r\n";
const ENTER_ALTERNATE_SCREEN: &str = "\x1b[?1049h";
/// The longest last line the agent list receives; the full text stays in the
/// terminal.
const LAST_LINE_CHARS: usize = 200;

/// Counts the bells the parser reports, so the session can turn each into one
/// event. The parser calls `audible_bell` only for a `0x07` outside an OSC
/// string: inside one, the byte ends the string instead (vt100 0.18.1,
/// vte 0.15.0).
#[derive(Default)]
struct BellCounter {
    count: usize,
}

impl vt100::Callbacks for BellCounter {
    fn audible_bell(&mut self, _screen: &mut vt100::Screen) {
        self.count += 1;
    }
}

/// The server-side copy of one terminal screen. It also answers the terminal
/// queries that the program asks, so the answer does not depend on a client.
pub struct ScreenEmulator {
    parser: vt100::Parser<BellCounter>,
    scanner: QueryScanner,
    size: TerminalSize,
}

impl ScreenEmulator {
    #[must_use]
    pub fn new(size: TerminalSize, scrollback_lines: usize) -> Self {
        Self {
            parser: vt100::Parser::new_with_callbacks(
                size.rows(),
                size.columns(),
                scrollback_lines,
                BellCounter::default(),
            ),
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

    /// The number of bells since the last call, which also clears the count.
    /// A bell and its delivery are separate reads, so the session owns the
    /// moment of the report.
    pub fn take_bells(&mut self) -> usize {
        std::mem::take(&mut self.parser.callbacks_mut().count)
    }

    /// The last line of the screen that shows anything, plain text: the
    /// parser has already applied every redraw, so color and cursor codes
    /// never reach this text. It is cut to `LAST_LINE_CHARS` characters on a
    /// character boundary. An empty screen yields `None`.
    #[must_use]
    pub fn last_line(&self) -> Option<String> {
        let line = self
            .parser
            .screen()
            .contents()
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())?
            .trim_end()
            .to_owned();
        Some(if line.chars().count() > LAST_LINE_CHARS {
            line.chars().take(LAST_LINE_CHARS).collect()
        } else {
            line
        })
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
