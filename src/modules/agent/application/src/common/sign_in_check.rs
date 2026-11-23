// SPDX-License-Identifier: AGPL-3.0-or-later

//! The rules of the sign-in check (0019): what a status command's end means,
//! and the texts that tell the person. No text here carries a status
//! command's output, except the one quoted line of a not-signed-in answer
//! (FR-007, FR-008).

use std::time::Duration;

use kernel::visible_text;

use crate::common::models::{SignIn, StatusCommand, StatusRunResult};

/// How long a status command may run before the start goes on without it
/// (0019 A3).
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5);

/// The most of a status command's output that the office keeps (FR-006).
pub const STATUS_OUTPUT_CAP_BYTES: usize = 64 * 1024;

/// The most characters of a quoted line, the cut mark included (FR-007).
pub const QUOTED_LINE_MAX_CHARS: usize = 200;

const CUT_MARK: char = '…';

/// What the sign-in check found for one start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInAnswer {
    SignedIn,
    /// `line` is the line that said so, quoted as FR-007 allows.
    NotSignedIn {
        line: String,
    },
    NoAnswer(NoAnswerReason),
    /// No check ran; the start goes on as if no status command were known.
    NotChecked,
}

/// Why the check could not tell (0019 A2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoAnswerReason {
    TimedOut(Duration),
    CouldNotRun(String),
    Unclear,
    /// The agent's command, folder, or additions changed between the check
    /// and the start (FR-004).
    AgentChanged,
}

/// How the office found that a CLI is not signed in.
#[derive(Debug, Clone, Copy)]
pub enum NotSignedInSource<'a> {
    StatusCommand(&'a StatusCommand),
    SignInScreen,
}

/// The answer of a status command's end: exit 0 is signed in; a non-zero
/// exit with one of the words is not signed in; everything else is no
/// answer, so a CLI without the subcommand never refuses a start (A2).
#[must_use]
pub fn answer(
    result: &StatusRunResult,
    command: &StatusCommand,
    timeout: Duration,
) -> SignInAnswer {
    match result {
        StatusRunResult::Ended {
            exit_code: Some(0), ..
        } => SignInAnswer::SignedIn,
        StatusRunResult::Ended { output, .. } => {
            match line_with_word(output, command.not_signed_in_words) {
                Some(line) => SignInAnswer::NotSignedIn {
                    line: quoted_line(line),
                },
                None => SignInAnswer::NoAnswer(NoAnswerReason::Unclear),
            }
        }
        StatusRunResult::TimedOut => SignInAnswer::NoAnswer(NoAnswerReason::TimedOut(timeout)),
        StatusRunResult::CouldNotRun(error) => {
            SignInAnswer::NoAnswer(NoAnswerReason::CouldNotRun(quoted_line(error)))
        }
        StatusRunResult::Skipped => SignInAnswer::NotChecked,
    }
}

/// The first line of `text` that holds one of the lower-case `words`,
/// compared without case.
#[must_use]
pub fn line_with_word<'a>(text: &'a str, words: &[&str]) -> Option<&'a str> {
    text.lines().find(|line| {
        let lowered = line.to_lowercase();
        words.iter().any(|word| lowered.contains(word))
    })
}

/// A line of a CLI, as the person may see it: trimmed, in visible form, and
/// at most [`QUOTED_LINE_MAX_CHARS`] characters long.
#[must_use]
pub fn quoted_line(line: &str) -> String {
    let visible = visible_text(line.trim());
    if visible.chars().count() <= QUOTED_LINE_MAX_CHARS {
        return visible;
    }
    let mut cut: String = visible.chars().take(QUOTED_LINE_MAX_CHARS - 1).collect();
    cut.push(CUT_MARK);
    cut
}

/// The one reason of a start that ends because the CLI is not signed in
/// (FR-007). `line` must already be a quoted line.
#[must_use]
pub fn not_signed_in_reason(sign_in: &SignIn, source: NotSignedInSource<'_>, line: &str) -> String {
    let cli = sign_in.cli;
    let found = match source {
        NotSignedInSource::StatusCommand(command) => {
            format!("`{cli} {}` said so", command.arguments.join(" "))
        }
        NotSignedInSource::SignInScreen => "its first screen asks to sign in".to_owned(),
    };
    format!(
        "{cli} is not signed in: {found}. Sign in to {cli} in your own terminal, \
         then start the agent again. It showed: {line}"
    )
}

/// The note of a start that went on because the check gave no answer
/// (FR-004). It never holds the command's output.
#[must_use]
pub fn no_answer_note(reason: &NoAnswerReason) -> String {
    let why = match reason {
        NoAnswerReason::TimedOut(timeout) => {
            format!("it timed out after {} s", timeout.as_secs())
        }
        NoAnswerReason::CouldNotRun(error) => format!("it could not run: {error}"),
        NoAnswerReason::Unclear => "its answer was unclear".to_owned(),
        NoAnswerReason::AgentChanged => "the agent changed during the check".to_owned(),
    };
    format!("the sign-in check gave no answer: {why}")
}

#[cfg(test)]
#[path = "sign_in_check.tests.rs"]
mod tests;
