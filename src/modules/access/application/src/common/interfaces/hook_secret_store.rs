// SPDX-License-Identifier: AGPL-3.0-or-later

use access_domain::value_objects::OfficeToken;
use kernel::AgentId;

/// Keeps the status-hook secrets of the running sessions (0024). A secret is
/// narrower than the office token and the agent call credential: it proves
/// only that a report comes from the CLI of this session, and it delivers
/// reports of that session and nothing else. Secrets live in memory only; a
/// server restart ends every session, so none outlives its server.
pub trait HookSecretStore: Send + Sync {
    /// Issues a new secret for the session, replacing any earlier one of the
    /// same session.
    ///
    /// # Errors
    /// `Randomness` when the system has no random bytes.
    fn issue(
        &self,
        agent_id: AgentId,
        session: &str,
    ) -> Result<OfficeToken, crate::common::errors::AccessError>;

    /// The agent of `session` when `presented` is the secret of that session.
    /// An unknown session, a revoked one, and a wrong secret all give `None`,
    /// and take the same work, so the caller cannot tell them apart.
    fn authorize(&self, session: &str, presented: &str) -> Option<AgentId>;

    /// Drops the secret of `session`.
    fn revoke_session(&self, session: &str);

    /// Drops the secrets of every session of the agent.
    fn revoke_agent(&self, agent_id: AgentId);
}
