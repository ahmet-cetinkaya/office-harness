// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::entities::{Agent, Session};
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{ExitResult, SessionId};
use kernel::AgentId;

use crate::common::action_journal::ActionJournal;
use crate::common::continuation::{adapter_of, continuation_of};
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AgentStore, CliAdapters, SessionStore};
use crate::common::lookup::require_agent;
use crate::common::models::AgentAction;
use crate::common::session_launcher::{LaunchKind, SessionLauncher, StartOutcome};

const NO_EARLIER_SESSION: &str = "the agent has no earlier session";
const NEW_SESSION_PREFIX: &str = "new session: ";
const MILLIS_PER_SECOND: i64 = 1000;

/// What `agent.resume` started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeOutcome {
    pub start: StartOutcome,
    /// True when the last session was continued, false for a new session.
    pub continued: bool,
    /// Why a new session started instead of a continue.
    pub reason: Option<String>,
}

/// Which request a continue or new session answers. It only picks the action
/// of the record: a resume records `continue` or `start`, a wake records `wake`.
#[derive(Clone, Copy)]
enum Flow {
    Resume,
    Wake,
}

impl Flow {
    fn action_of(self, continued: bool) -> AgentAction {
        match (self, continued) {
            (Self::Wake, _) => AgentAction::Wake,
            (Self::Resume, true) => AgentAction::Continue,
            (Self::Resume, false) => AgentAction::Start,
        }
    }
}

/// Resumes an interrupted agent (`agent.resume`, spec 0013): it continues the
/// last session when the CLI can, and otherwise starts a new one. The check
/// and the start are one step, so the host runs it under the status order and
/// no start can come between them.
#[derive(Clone)]
pub struct ResumeAgentHandler {
    agents: Arc<dyn AgentStore>,
    sessions: Arc<dyn SessionStore>,
    adapters: Arc<dyn CliAdapters>,
    launcher: SessionLauncher,
    journal: ActionJournal,
}

impl ResumeAgentHandler {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        sessions: Arc<dyn SessionStore>,
        adapters: Arc<dyn CliAdapters>,
        launcher: SessionLauncher,
        journal: ActionJournal,
    ) -> Self {
        Self {
            agents,
            sessions,
            adapters,
            launcher,
            journal,
        }
    }

    /// # Errors
    /// `NotFound` for an unknown agent, `Conflict` when the agent is no longer
    /// interrupted, and the errors of a start.
    pub fn resume(&self, agent_id: AgentId) -> Result<ResumeOutcome, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        if agent.status != AgentStatus::Interrupted {
            return Err(ApplicationError::Conflict(
                "the agent is no longer interrupted".to_owned(),
            ));
        }
        self.continue_or_start_new(&agent, Flow::Resume)
    }

    /// Wakes an asleep agent (`agent.wake`, spec 0018): the same continue or
    /// new session as `resume`, recorded as a wake.
    ///
    /// # Errors
    /// `NotFound` for an unknown agent, `Conflict` when the agent is no longer
    /// asleep, and the errors of a start.
    pub fn wake(&self, agent_id: AgentId) -> Result<ResumeOutcome, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        if agent.status != AgentStatus::Asleep {
            return Err(ApplicationError::Conflict(
                "the agent is no longer asleep".to_owned(),
            ));
        }
        self.continue_or_start_new(&agent, Flow::Wake)
    }

    fn continue_or_start_new(
        &self,
        agent: &Agent,
        flow: Flow,
    ) -> Result<ResumeOutcome, ApplicationError> {
        let last = self.sessions.latest_for_agent(agent.id)?;
        let adapter = adapter_of(&*self.adapters, &agent.command)?;
        let continuation = match &last {
            Some(session) => continuation_of(session, &*adapter).map(|id| (session, id)),
            None => Err(NO_EARLIER_SESSION.to_owned()),
        };
        match continuation {
            Ok((from, cli_session_id)) => {
                let kind = LaunchKind::Continue {
                    from,
                    cli_session_id,
                };
                let start = self.launch_and_record(agent.id, &kind, flow.action_of(true), None)?;
                Ok(ResumeOutcome {
                    start,
                    continued: true,
                    reason: None,
                })
            }
            Err(reason) => self.start_new_session(agent.id, reason, flow),
        }
    }

    /// Starts a new session after a continue that the CLI refused: the
    /// refused session must still be the agent's last, and the agent must have
    /// ended on its own. The caller sets the actor of the record.
    ///
    /// # Errors
    /// `NotFound` for an unknown agent, `Conflict` when the agent changed
    /// after the refused session, and the errors of a start.
    pub fn fall_back(
        &self,
        agent_id: AgentId,
        after_session: SessionId,
    ) -> Result<ResumeOutcome, ApplicationError> {
        let agent = require_agent(&*self.agents, agent_id)?;
        let ended_on_its_own = matches!(agent.status, AgentStatus::Exited | AgentStatus::Failed);
        let refused = self
            .sessions
            .latest_for_agent(agent_id)?
            .filter(|session| session.id == after_session);
        let reason = refused.as_ref().and_then(refusal_reason);
        let (true, Some(reason)) = (ended_on_its_own, reason) else {
            return Err(ApplicationError::Conflict(
                "the agent changed after the refused session".to_owned(),
            ));
        };
        self.start_new_session(agent_id, reason, Flow::Resume)
    }

    /// A new session in place of a continue, with the reason in the record.
    fn start_new_session(
        &self,
        agent_id: AgentId,
        reason: String,
        flow: Flow,
    ) -> Result<ResumeOutcome, ApplicationError> {
        let detail = format!("{NEW_SESSION_PREFIX}{reason}");
        let start = self.launch_and_record(
            agent_id,
            &LaunchKind::NewTask,
            flow.action_of(false),
            Some(&detail),
        )?;
        Ok(ResumeOutcome {
            start,
            continued: false,
            reason: Some(reason),
        })
    }

    /// Launches and writes the action record with the failure of the start,
    /// as `agent.start` does.
    fn launch_and_record(
        &self,
        agent_id: AgentId,
        kind: &LaunchKind<'_>,
        action: AgentAction,
        detail: Option<&str>,
    ) -> Result<StartOutcome, ApplicationError> {
        let mut agent = require_agent(&*self.agents, agent_id)?;
        let outcome = self.launcher.launch(&mut agent, kind);
        let failure = match &outcome {
            Ok(outcome) => outcome.failure.clone(),
            Err(error) => Some(error.to_string()),
        };
        self.journal
            .record_detailed(agent_id, action, failure.as_deref(), detail)?;
        outcome
    }
}

/// Why a new session replaces a refused continue, built from the refused
/// session only. `None` when the session has not ended.
fn refusal_reason(session: &Session) -> Option<String> {
    let ended_at = session.ended_at?;
    let result = session.exit_result.as_ref()?;
    if let ExitResult::SpawnError(text) = result {
        return Some(format!("the continue did not start: {text}"));
    }
    let seconds =
        (ended_at.as_unix_millis() - session.started_at.as_unix_millis()) / MILLIS_PER_SECOND;
    let outcome = match result {
        ExitResult::Code(code) => format!("exit code {code}"),
        other => format!("result {other}"),
    };
    Some(format!(
        "the CLI ended {seconds} s after the continue, with {outcome}"
    ))
}

#[cfg(test)]
#[path = "resume_agent_handler.tests.rs"]
mod tests;
