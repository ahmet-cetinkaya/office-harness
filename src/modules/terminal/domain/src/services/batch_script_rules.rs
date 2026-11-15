// SPDX-License-Identifier: AGPL-3.0-or-later

//! Rules for Windows batch scripts (`.cmd`, `.bat`). `cmd.exe` cannot escape
//! these characters safely (the CVE-2024-24576 class of flaw), and the PTY
//! library builds the command line itself, so the server refuses them.

use std::path::Path;

const BATCH_EXTENSIONS: [&str; 2] = ["cmd", "bat"];
/// The library quotes an argument only for a space, a tab, a line feed, a
/// vertical tab, or a double quote. Every other character here reaches
/// `cmd.exe` as written, and `cmd.exe` acts on it: `%` and `!` expand
/// variables, `&`, `|`, `<`, `>` and `^` chain, redirect or escape, and the
/// parentheses group commands.
const UNSAFE_CHARACTERS: [char; 12] =
    ['%', '!', '"', '\r', '\n', '&', '|', '<', '>', '^', '(', ')'];

#[must_use]
pub fn is_batch_script(program: &str) -> bool {
    Path::new(program).extension().is_some_and(|extension| {
        BATCH_EXTENSIONS
            .iter()
            .any(|batch| extension.eq_ignore_ascii_case(batch))
    })
}

/// The first character of `text` that a batch script cannot receive safely.
#[must_use]
pub fn first_unsafe_batch_character(text: &str) -> Option<char> {
    text.chars()
        .find(|character| UNSAFE_CHARACTERS.contains(character))
}

/// The index and the character of the first argument that a batch script
/// cannot receive safely.
#[must_use]
pub fn first_unsafe_batch_argument(arguments: &[String]) -> Option<(usize, char)> {
    arguments.iter().enumerate().find_map(|(index, argument)| {
        first_unsafe_batch_character(argument).map(|character| (index, character))
    })
}

#[cfg(test)]
#[path = "batch_script_rules.tests.rs"]
mod tests;
