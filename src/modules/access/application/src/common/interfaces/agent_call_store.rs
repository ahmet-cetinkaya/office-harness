// SPDX-License-Identifier: AGPL-3.0-or-later

use access_domain::value_objects::OfficeToken;
use kernel::AgentId;

/// The call credential of one running session of an agent (0002). It is a
/// random token like the office token, but it proves only that the caller is
/// this agent in this session.
#[derive(Clone, PartialEq, Eq)]
pub struct AgentCredential {
    pub token: OfficeToken,
    /// The session the token was issued for. A new session gets a new token,
    /// so a token of an earlier session never works again.
    pub session: String,
}

impl std::fmt::Debug for AgentCredential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentCredential")
            .field("session", &self.session)
            .finish_non_exhaustive()
    }
}

/// Keeps the call credentials of the running agents. They live in memory
/// only: a server restart ends every session, so no credential outlives one.
pub trait AgentCallStore: Send + Sync {
    /// Issues a new credential for the session, replacing any earlier one of
    /// the agent.
    ///
    /// # Errors
    /// `Randomness` when the system has no random bytes.
    fn issue(
        &self,
        agent_id: AgentId,
        session: &str,
    ) -> Result<AgentCredential, crate::common::errors::AccessError>;

    /// The credential of the agent's running session, if it has one.
    fn current(&self, agent_id: AgentId) -> Option<AgentCredential>;

    /// Drops the credential of `session`. A credential of a newer session of
    /// the same agent stays.
    fn revoke_session(&self, agent_id: AgentId, session: &str);

    /// Drops the credential of the agent, whatever its session.
    fn revoke_agent(&self, agent_id: AgentId);
}
