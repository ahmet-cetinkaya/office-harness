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

/// The answer for the line at the cursor after the interrupt key, only when
/// the line changed since before the key. A question that was already on the
/// screen belongs to the program itself, so it never gets an answer.
#[must_use]
pub fn answer_after_interrupt(line_before_interrupt: &str, line: &str) -> Option<char> {
    if line == line_before_interrupt {
        return None;
    }
    confirmation_answer(line)
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
