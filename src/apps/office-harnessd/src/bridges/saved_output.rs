// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::SessionStore;
use kernel::AgentId;
use terminal_domain::value_objects::OutputKey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SavedOutputError {
    #[error("the sessions cannot be read: {0}")]
    Sessions(String),
    #[error(transparent)]
    Key(#[from] terminal_domain::errors::TerminalError),
}

/// Names the output file of an agent's last session, which `terminal.attach`
/// shows when the agent does not run. The terminal module knows files by
/// key; the agent module knows which session is the last.
#[derive(Clone)]
pub struct SavedOutputs {
    sessions: Arc<dyn SessionStore>,
}

impl SavedOutputs {
    #[must_use]
    pub fn new(sessions: Arc<dyn SessionStore>) -> Self {
        Self { sessions }
    }

    /// `None` when the agent never had a session.
    ///
    /// # Errors
    /// `Sessions` when the store fails, or `Key` for an id that is not a
    /// file name.
    pub fn last_session_output(
        &self,
        agent_id: AgentId,
    ) -> Result<Option<OutputKey>, SavedOutputError> {
        let latest = self
            .sessions
            .latest_for_agent(agent_id)
            .map_err(|error| SavedOutputError::Sessions(error.to_string()))?;
        Ok(latest
            .map(|session| OutputKey::parse(&session.id.to_string()))
            .transpose()?)
    }
}
