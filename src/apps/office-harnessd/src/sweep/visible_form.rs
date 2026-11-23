// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write;
use std::ops::RangeInclusive;

use terminal_application::common::models::COMMAND_DISPLAY_LIMIT;

const ELLIPSIS: char = '…';
/// Four bytes of the hash are the 8 hex characters that name a longer line.
const HASH_PREFIX_BYTES: usize = 4;

/// The bidirectional format characters: they reorder the text around them.
/// `char::is_control` does not cover them.
const BIDI_CONTROLS: [RangeInclusive<u32>; 4] = [
    0x061c..=0x061c,
    0x200e..=0x200f,
    0x202a..=0x202e,
    0x2066..=0x2069,
];

/// The command line in visible form (0017 Terms): each control, escape, and
/// bidirectional-control character is written as `\u{..}`. A stored line that
/// reached the display limit was cut, so it ends with an ellipsis and the
/// first 8 hex characters of the hash of the whole line.
#[must_use]
pub fn visible_command(display: &str, command_hash: &[u8; 32]) -> String {
    let mut visible = escape_unsafe(display);
    if display.chars().count() >= COMMAND_DISPLAY_LIMIT {
        visible.push(ELLIPSIS);
        for byte in &command_hash[..HASH_PREFIX_BYTES] {
            // Writing into a String cannot fail.
            let _ = write!(visible, "{byte:02x}");
        }
    }
    visible
}

/// The text with every control and bidirectional-control character written
/// as `\u{..}`, so nothing in it acts on the reader's terminal or reorders
/// what the reader sees.
#[must_use]
pub fn escape_unsafe(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if is_unsafe(character) {
            let _ = write!(escaped, "\\u{{{:x}}}", u32::from(character));
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn is_unsafe(character: char) -> bool {
    let code = u32::from(character);
    character.is_control() || BIDI_CONTROLS.iter().any(|range| range.contains(&code))
}

#[cfg(test)]
#[path = "visible_form.tests.rs"]
mod tests;
