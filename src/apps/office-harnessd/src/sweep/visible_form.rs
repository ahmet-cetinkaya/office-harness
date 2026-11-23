// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write;
use std::ops::RangeInclusive;

use terminal_application::common::models::COMMAND_DISPLAY_LIMIT;

const ELLIPSIS: char = '…';
/// Four bytes of the hash are the 8 hex characters that name a longer line.
const HASH_PREFIX_BYTES: usize = 4;

/// The Unicode format characters (category Cf), which `char::is_control` does
/// not cover: the bidirectional controls that reorder the text around them,
/// the zero-width characters that hide text, and the soft hyphen. The desktop
/// escapes the same set (`\p{Cf}`); the standard library has no category
/// lookup, so the ranges are listed here.
const FORMAT_CHARACTERS: [RangeInclusive<u32>; 22] = [
    0x00ad..=0x00ad,
    0x0600..=0x0605,
    0x061c..=0x061c,
    0x06dd..=0x06dd,
    0x070f..=0x070f,
    0x0890..=0x0891,
    0x08e2..=0x08e2,
    0x180e..=0x180e,
    0x200b..=0x200f,
    0x202a..=0x202e,
    0x2060..=0x2064,
    0x2066..=0x206f,
    0xfeff..=0xfeff,
    0xfff9..=0xfffb,
    0x110bd..=0x110bd,
    0x110cd..=0x110cd,
    0x13430..=0x1343f,
    0x1bca0..=0x1bca3,
    0x1d173..=0x1d17a,
    0xe0001..=0xe0001,
    0xe0020..=0xe007f,
    // The line and paragraph separators (category Zl and Zp) break a line in
    // a viewer that renders text.
    0x2028..=0x2029,
];

/// The command line in visible form (0017 Terms): each control, escape, format, and
/// line-separator character is written as `\u{..}`. A stored line that
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

/// The text with every control and format character (zero-width and
/// bidirectional ones included) written as `\u{..}`, so nothing in it acts on
/// the reader's terminal, hides text, or reorders what the reader sees.
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
    character.is_control() || FORMAT_CHARACTERS.iter().any(|range| range.contains(&code))
}

#[cfg(test)]
#[path = "visible_form.tests.rs"]
mod tests;
