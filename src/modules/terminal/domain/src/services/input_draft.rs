// SPDX-License-Identifier: AGPL-3.0-or-later

const ESCAPE: char = '\u{1b}';
const BACKSPACE: char = '\u{8}';
const DELETE: char = '\u{7f}';
/// Ctrl+C and Ctrl+U: the keys that clear a CLI's input line.
const LINE_CLEARING_KEYS: [char; 2] = ['\u{3}', '\u{15}'];
const ENTER_KEYS: [char; 2] = ['\r', '\n'];
const PASTE_START: &str = "200";
const PASTE_END: &str = "201";
/// The last characters of the escape sequences a terminal sends for a
/// gained or lost focus and for a mouse report: they edit no text.
const NON_EDITING_FINALS: [char; 4] = ['I', 'O', 'M', 'm'];

/// Where the reading stands inside an escape sequence, which may be split
/// across two inputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Escape {
    #[default]
    None,
    /// After the escape character.
    Started,
    /// Inside `ESC [`, with the parameters read so far.
    ControlSequence(String),
    /// After `ESC O`: one more character ends it.
    SingleShift,
}

/// An estimate of the text a person typed at a CLI's input line and has not
/// sent or cleared (0012 A5), read from the keys alone. It counts typed
/// characters, takes one off for an erase key, and returns to zero on Enter
/// or a line-clearing key. A key that edits in a way it cannot follow, such
/// as an arrow or a word erase, keeps it above zero, so an error only holds
/// directives longer and never types after a person's text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputDraft {
    characters: usize,
    in_paste: bool,
    escape: Escape,
}

impl InputDraft {
    /// True while the person may have text at the input line that was not
    /// sent or cleared. An escape that no sequence followed is a lone Escape
    /// key, whose effect on the line is unknown.
    #[must_use]
    pub fn is_unsent(&self) -> bool {
        self.characters > 0 || self.escape != Escape::None
    }

    /// Reads one input that a person's client sent to the terminal.
    pub fn feed(&mut self, bytes: &[u8]) {
        for character in String::from_utf8_lossy(bytes).chars() {
            self.read(character);
        }
    }

    fn read(&mut self, character: char) {
        match std::mem::take(&mut self.escape) {
            Escape::None => self.read_key(character),
            Escape::Started => self.read_after_escape(character),
            Escape::ControlSequence(parameters) => {
                self.read_control_sequence(parameters, character);
            }
            // `ESC O` and one character: a cursor or function key.
            Escape::SingleShift => self.keep_unsent(),
        }
    }

    fn read_key(&mut self, character: char) {
        if character == ESCAPE {
            self.escape = Escape::Started;
        } else if self.sends_or_clears(character) {
            self.characters = 0;
        } else if character == BACKSPACE || character == DELETE {
            self.characters = self.characters.saturating_sub(1);
        } else if character == '\t' || !character.is_control() || self.in_paste {
            self.characters += 1;
        } else {
            // Another control key, such as a word erase: its effect on the
            // line is unknown, so the line stays held.
            self.keep_unsent();
        }
    }

    /// Enter outside a paste sends the line; a line-clearing key empties it.
    fn sends_or_clears(&self, character: char) -> bool {
        (ENTER_KEYS.contains(&character) && !self.in_paste)
            || LINE_CLEARING_KEYS.contains(&character)
    }

    fn read_after_escape(&mut self, character: char) {
        match character {
            '[' => self.escape = Escape::ControlSequence(String::new()),
            'O' => self.escape = Escape::SingleShift,
            // A lone escape, or an Alt chord: unknown effect on the line.
            _ => self.keep_unsent(),
        }
    }

    fn read_control_sequence(&mut self, mut parameters: String, character: char) {
        if !('@'..='~').contains(&character) {
            parameters.push(character);
            self.escape = Escape::ControlSequence(parameters);
            return;
        }
        if character == '~' && parameters == PASTE_START {
            self.in_paste = true;
        } else if character == '~' && parameters == PASTE_END {
            self.in_paste = false;
        } else if !NON_EDITING_FINALS.contains(&character) {
            // An arrow, Home, End, or Delete key moves or edits the line in
            // a way this count cannot follow.
            self.keep_unsent();
        }
    }

    fn keep_unsent(&mut self) {
        self.characters = self.characters.max(1);
    }
}

#[cfg(test)]
#[path = "input_draft.tests.rs"]
mod tests;
