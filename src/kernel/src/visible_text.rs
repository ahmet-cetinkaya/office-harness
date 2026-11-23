// SPDX-License-Identifier: AGPL-3.0-or-later

//! The visible form of a text that crosses from one agent to another or to
//! the person, or that a CLI printed: no control or bidirectional format
//! character travels as itself (0014, 0015/FR-003, 0019/FR-007).

use std::fmt::Write as _;

/// The last C0 control character; U+0000 to here map to the control pictures.
const C0_END: u32 = 0x1f;
const CONTROL_PICTURES_START: u32 = 0x2400;
const DEL: u32 = 0x7f;
const DEL_PICTURE: char = '␡';
const C1_START: u32 = 0x80;
const C1_END: u32 = 0x9f;

/// Bidirectional format characters: they reorder the text around them, so a
/// reader could take the words in another order than they were written.
const BIDI_EMBEDDINGS_START: u32 = 0x202a;
const BIDI_EMBEDDINGS_END: u32 = 0x202e;
const BIDI_ISOLATES_START: u32 = 0x2066;
const BIDI_ISOLATES_END: u32 = 0x2069;

/// The text with every control and bidirectional format character shown as
/// a visible symbol, as the desktop's task line shows it, so nothing in it
/// acts on the reader's terminal or reorders what it reads (0011/FR-006).
#[must_use]
pub fn visible_text(text: &str) -> String {
    let mut visible = String::with_capacity(text.len());
    for character in text.chars() {
        push_visible(&mut visible, character);
    }
    visible
}

fn push_visible(visible: &mut String, character: char) {
    let code = u32::from(character);
    if code <= C0_END {
        let picture = char::from_u32(CONTROL_PICTURES_START + code);
        visible.push(picture.unwrap_or(char::REPLACEMENT_CHARACTER));
    } else if code == DEL {
        visible.push(DEL_PICTURE);
    } else if is_escaped(code) {
        // Writing into a String cannot fail.
        let _ = write!(visible, "\\u{code:04X}");
    } else {
        visible.push(character);
    }
}

/// C1 controls and the bidirectional format characters, which have no
/// picture of their own and are written out as their code.
fn is_escaped(code: u32) -> bool {
    (C1_START..=C1_END).contains(&code)
        || (BIDI_EMBEDDINGS_START..=BIDI_EMBEDDINGS_END).contains(&code)
        || (BIDI_ISOLATES_START..=BIDI_ISOLATES_END).contains(&code)
}

#[cfg(test)]
#[path = "visible_text.tests.rs"]
mod tests;
