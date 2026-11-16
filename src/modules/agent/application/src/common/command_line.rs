// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::AgentCommand;

use crate::common::errors::ApplicationError;

const QUOTE: char = '"';
const ESCAPE: char = '\\';

/// Splits the command text into the program and its fixed arguments, by the
/// rule of the Windows C runtime on every platform.
///
/// White space outside double quotes separates words, and a pair of double
/// quotes wraps text that holds white space (an empty pair is an empty
/// argument). A backslash is literal, unless a run of backslashes ends at a
/// double quote: then each pair of backslashes stands for one backslash, and
/// an odd one out makes the quote a literal quote. So paths like
/// `\\server\share\tool` and `C:\Program Files\x` need no escaping, and a
/// quoted path that ends in a backslash writes it twice, as in `"C:\dir\\"`.
/// Nothing else is interpreted: no variables, no globs.
///
/// # Errors
/// `Invalid` for a quote that is never closed, or a command whose first
/// word is empty.
pub fn split_command(command: &AgentCommand) -> Result<(String, Vec<String>), ApplicationError> {
    let mut words = split_words(command.as_str())?.into_iter();
    let program = words.next().filter(|program| !program.is_empty());
    let Some(program) = program else {
        return Err(ApplicationError::Invalid(
            "command must start with a program name".to_owned(),
        ));
    };
    Ok((program, words.collect()))
}

fn split_words(text: &str) -> Result<Vec<String>, ApplicationError> {
    let characters: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut in_quotes = false;
    let mut index = 0;
    while let Some(&character) = characters.get(index) {
        match character {
            ESCAPE => {
                let run = characters[index..]
                    .iter()
                    .take_while(|&&next| next == ESCAPE)
                    .count();
                index += run;
                in_word = true;
                if characters.get(index) == Some(&QUOTE) {
                    word.extend(std::iter::repeat_n(ESCAPE, run / 2));
                    if run % 2 == 1 {
                        word.push(QUOTE);
                        index += 1;
                    }
                } else {
                    word.extend(std::iter::repeat_n(ESCAPE, run));
                }
            }
            QUOTE => {
                in_quotes = !in_quotes;
                in_word = true;
                index += 1;
            }
            space if space.is_whitespace() && !in_quotes => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
                index += 1;
            }
            other => {
                word.push(other);
                in_word = true;
                index += 1;
            }
        }
    }
    if in_quotes {
        return Err(ApplicationError::Invalid(
            "command has a double quote that is never closed".to_owned(),
        ));
    }
    if in_word {
        words.push(word);
    }
    Ok(words)
}

#[cfg(test)]
#[path = "command_line.tests.rs"]
mod tests;
