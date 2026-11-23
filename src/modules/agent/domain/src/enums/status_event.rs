// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

/// What happened to an agent's session. Each event is one trigger of the
/// status state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusEvent {
    Start,
    FirstOutput,
    QuietPeriodElapsed,
    Output,
    ExitedWithCodeZero,
    ExitedWithFailure,
    StoppedByUser,
    ServerStopped,
    PutToSleep,
}

impl StatusEvent {
    pub const ALL: [Self; 9] = [
        Self::Start,
        Self::FirstOutput,
        Self::QuietPeriodElapsed,
        Self::Output,
        Self::ExitedWithCodeZero,
        Self::ExitedWithFailure,
        Self::StoppedByUser,
        Self::ServerStopped,
        Self::PutToSleep,
    ];
}

impl fmt::Display for StatusEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Start => "start",
            Self::FirstOutput => "first output",
            Self::QuietPeriodElapsed => "quiet period elapsed",
            Self::Output => "output",
            Self::ExitedWithCodeZero => "exit with code 0",
            Self::ExitedWithFailure => "exit with failure",
            Self::StoppedByUser => "stop by the user",
            Self::ServerStopped => "server stop",
            Self::PutToSleep => "sleep after the idle time",
        };
        formatter.write_str(name)
    }
}
