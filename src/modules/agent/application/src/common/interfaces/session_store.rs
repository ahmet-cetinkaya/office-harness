// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Session;
use agent_domain::value_objects::{ExitResult, SessionId};
use kernel::{AgentId, Timestamp};

use crate::common::errors::PortError;

/// The stored sessions. Deleting an agent deletes its sessions.
pub trait SessionStore: Send + Sync {
    /// # Errors
    /// `Storage`.
    fn insert(&self, session: &Session) -> Result<(), PortError>;

    /// # Errors
    /// `Storage`.
    fn find(&self, id: SessionId) -> Result<Option<Session>, PortError>;

    /// The agent's sessions, newest first.
    ///
    /// # Errors
    /// `Storage`.
    fn list_for_agent(&self, agent_id: AgentId) -> Result<Vec<Session>, PortError>;

    /// # Errors
    /// `Storage`.
    fn latest_for_agent(&self, agent_id: AgentId) -> Result<Option<Session>, PortError>;

    /// The sessions that have not ended.
    ///
    /// # Errors
    /// `Storage`.
    fn running(&self) -> Result<Vec<Session>, PortError>;

    /// # Errors
    /// `NotFound` when the session does not exist.
    fn set_cli_session_id(&self, id: SessionId, cli_session_id: &str) -> Result<(), PortError>;

    /// Ends the agent's running session. Returns false when it has none, so
    /// a second call for the same end changes nothing.
    ///
    /// # Errors
    /// `Storage`.
    fn end_running(
        &self,
        agent_id: AgentId,
        result: &ExitResult,
        at: Timestamp,
    ) -> Result<bool, PortError>;
}
