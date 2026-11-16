// SPDX-License-Identifier: AGPL-3.0-or-later

/// The key that answers a question of the form `(X/Y)?` at the end of the
/// line, such as the "Terminate batch job (Y/N)?" of `cmd.exe`, in any
/// language. It is the first letter, which is the "yes" of the question.
/// `None` when the line does not end in such a question, so nothing is typed
/// blindly.
#[must_use]
pub fn confirmation_answer(line: &str) -> Option<char> {
    let choices = line.trim_end().strip_suffix(")?")?.rsplit_once('(')?.1;
    let (yes, no) = choices.split_once('/')?;
    single_letter(no)?;
    single_letter(yes)
}

fn single_letter(text: &str) -> Option<char> {
    let mut characters = text.chars();
    let letter = characters
        .next()
        .filter(|character| character.is_alphabetic())?;
    characters.next().is_none().then_some(letter)
}

#[cfg(test)]
#[path = "batch_confirmation.tests.rs"]
mod tests;
