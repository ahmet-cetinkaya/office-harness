// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::time::Duration;

/// What the office knows about one CLI's sign-in (0019, research R1, R4).
/// The default knows nothing: no status command and no sign-in screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SignIn {
    /// The CLI's name as the person types it, for the reason text.
    pub cli: &'static str,
    /// The read-only command that tells whether the CLI is signed in.
    pub status: Option<StatusCommand>,
    /// Words of the CLI's sign-in screen, in lower case.
    pub screen_words: &'static [&'static str],
}

/// A CLI's read-only sign-in status command (0019 A1, A2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCommand {
    pub arguments: &'static [&'static str],
    /// Words, in lower case, that the command prints when the CLI is not
    /// signed in.
    pub not_signed_in_words: &'static [&'static str],
    /// Variables that sign the CLI in where the status command may not see
    /// it; when one is set, no check runs (0019/FR-010, A11).
    pub key_variables: &'static [&'static str],
}

/// One run of a status command, with what the agent's session would get:
/// the program as the agent's command names it, the working folder, and
/// the office additions (0019/FR-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRun {
    pub program: String,
    pub arguments: Vec<String>,
    pub working_folder: PathBuf,
    pub environment_additions: Vec<(String, String)>,
    /// The CLI's sign-in key variables: a runner that finds one set in the
    /// login environment does not run the command.
    pub key_variables: &'static [&'static str],
    pub timeout: Duration,
}

/// How a status command ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusRunResult {
    /// The command ended by itself. `exit_code` is `None` when a signal
    /// ended it. `output` is what it printed, both streams, at most the
    /// output cap, with invalid bytes replaced.
    Ended {
        exit_code: Option<i32>,
        output: String,
    },
    /// The command did not end in time and was ended with all it started.
    TimedOut,
    /// The command could not start; the text says why.
    CouldNotRun(String),
    /// The command did not run, because a sign-in key variable is set in
    /// the login environment (0019/FR-010).
    Skipped,
}
