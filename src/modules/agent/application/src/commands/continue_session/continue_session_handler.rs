// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::{Agent, Session};
use agent_domain::value_objects::SessionId;

use crate::common::action_journal::ActionJournal;
use crate::common::continuation::{adapter_of, continuation_of};
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, CliAdapters, SessionStore};
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;
use crate::common::session_ender::SessionEnder;
use crate::common::session_launcher::{LaunchKind, SessionLauncher, StartOutcome};

/// Starts the CLI so that it continues a previous session
/// (`session.continue`, FR-025). The new session records which one it
/// continued. A running session of the agent ends first.
#[derive(Clone)]
pub struct ContinueSessionHandler {
    agents: Arc<dyn AgentStore>,
    sessions: Arc<dyn SessionStore>,
    adapters: Arc<dyn CliAdapters>,
    ender: SessionEnder,
    launcher: SessionLauncher,
    journal: ActionJournal,
}

impl ContinueSessionHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        sessions: Arc<dyn SessionStore>,
        adapters: Arc<dyn CliAdapters>,
        ender: SessionEnder,
        launcher: SessionLauncher,
        journal: ActionJournal,
    ) -> Self {
        Self {
            agents,
            sessions,
            adapters,
            ender,
            launcher,
            journal,
        }
    }

    /// # Errors
    /// `NotFound` for an unknown session, `Refused` with the reason when the
    /// session cannot be continued, and the errors of a start.
    pub fn handle(&self, session_id: SessionId) -> Result<StartOutcome, ApplicationError> {
        let session = self.sessions.find(session_id)?.ok_or_else(|| {
            ApplicationError::NotFound(format!("session {session_id} was not found"))
        })?;
        let agent = require_agent(&*self.agents, session.agent_id)?;
        let agent_id = agent.id;
        let outcome = self.begin(agent, &session);
        let failure = match &outcome {
            Ok(outcome) => outcome.failure.clone(),
            Err(error) => Some(error.to_string()),
        };
        self.journal
            .record(agent_id, AgentAction::Continue, failure.as_deref())?;
        outcome
    }

    fn begin(&self, mut agent: Agent, session: &Session) -> Result<StartOutcome, ApplicationError> {
        let adapter = adapter_of(&*self.adapters, &agent.command)?;
        // Checked, like the launch, before the running session ends, so a
        // refusal loses nothing.
        let cli_session_id =
            continuation_of(session, &*adapter).map_err(ApplicationError::Refused)?;
        let kind = LaunchKind::Continue {
            from: session,
            cli_session_id,
        };
        let prepared = self.launcher.prepare(&agent, &kind)?;
        if agent.status.is_running() {
            self.ender.stop_for_restart(&mut agent)?;
        }
        self.launcher.start(&mut agent, &kind, prepared)
    }
}

#[cfg(test)]
#[path = "continue_session_handler.tests.rs"]
mod tests;
