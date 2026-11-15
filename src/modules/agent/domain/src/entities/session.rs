// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{AgentId, Timestamp};

use crate::value_objects::{ExitResult, OfficeAdditions, SessionId};

/// One run of an agent's CLI, from start to end (FR-024).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: SessionId,
    pub agent_id: AgentId,
    /// The CLI's own session id. Empty for a generic command, and for a CLI
    /// whose id could not be matched to this session without a guess.
    pub cli_session_id: Option<String>,
    /// Set only when the user explicitly continued a session (FR-025).
    pub continued_from: Option<SessionId>,
    /// The office additions used at start (FR-023, SC-009).
    pub additions_snapshot: OfficeAdditions,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub exit_result: Option<ExitResult>,
}

impl Session {
    /// A session that just started.
    #[must_use]
    pub fn begin(
        id: SessionId,
        agent_id: AgentId,
        cli_session_id: Option<String>,
        continued_from: Option<SessionId>,
        additions_snapshot: OfficeAdditions,
        started_at: Timestamp,
    ) -> Self {
        Self {
            id,
            agent_id,
            cli_session_id,
            continued_from,
            additions_snapshot,
            started_at,
            ended_at: None,
            exit_result: None,
        }
    }

    pub fn end(&mut self, result: ExitResult, at: Timestamp) {
        self.exit_result = Some(result);
        self.ended_at = Some(at);
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        self.ended_at.is_none()
    }
}

/// The stored office additions of one agent, and when they last changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdditionsRecord {
    pub additions: OfficeAdditions,
    pub updated_at: Timestamp,
}

#[cfg(test)]
#[path = "session.tests.rs"]
mod tests;
