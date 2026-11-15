// SPDX-License-Identifier: AGPL-3.0-or-later

use super::server_file::Incompatibility;
use super::state::ConnectionState;

/// Why one attempt to connect failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No server answered, or the one that answered did not prove itself.
    NotReachable,
    /// The server closed with 1013: every handshake slot was busy. The token
    /// may be good, so the app says the server is busy and tries again.
    Busy,
    /// The app has no token to prove with.
    TokenUnavailable,
    /// The server closed with the unauthorized code.
    Rejected,
    Incompatible(Incompatibility),
}

/// The state to show after a failed attempt to connect.
pub fn state_after_failure(failure: Failure) -> ConnectionState {
    match failure {
        Failure::NotReachable => ConnectionState::NotReachable,
        Failure::Busy => ConnectionState::Busy,
        Failure::TokenUnavailable | Failure::Rejected => ConnectionState::NotAuthorized,
        Failure::Incompatible(incompatibility) => incompatibility.into(),
    }
}

#[cfg(test)]
#[path = "state_tracker.tests.rs"]
mod tests;
