// SPDX-License-Identifier: AGPL-3.0-or-later

/// What a terminal reports about its child. The agent bridge turns these into
/// status changes; the terminal module knows nothing about agents' meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalEvent {
    /// The child wrote its first output after the start.
    FirstOutput,
    /// The child wrote output, and then nothing for the quiet period.
    QuietPeriod,
    /// The child wrote output again after a `QuietPeriod`. It is sent once for
    /// each change from quiet to active, not for each chunk of output.
    OutputResumed,
    /// The child ended with this exit code.
    Exited { exit_code: u32 },
    /// The output file could not be written. The child keeps running.
    OutputStorageFailed,
}
