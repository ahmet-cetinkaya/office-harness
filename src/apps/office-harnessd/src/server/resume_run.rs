// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::AgentStore;
use agent_infrastructure::persistence::SessionProcessRepository;
use hierarchy_application::HoldService;
use kernel::{Actor, AgentId};
use serde_json::{Map, Value, json};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::task::spawn_blocking;
use tokio::time::{Instant, timeout_at};
use wire::{ErrorCode, Event, Request, Response};

use super::agent_requests::AgentGateway;
use super::event_hub::EventHub;
use crate::bootstrap::StorageAlerts;

const RESUME_REQUEST: &str = "agent.resume";
const WAKE_REQUEST: &str = "agent.wake";
const RESUME_RESULT_EVENT: &str = "resume.result";
const WAKE_RESULT_EVENT: &str = "wake.result";
const STATUS_EVENT: &str = "agent.status";
/// Requests that the server sends itself never reach a client, so their id
/// is never matched with an answer.
const OWN_REQUEST_ID: u32 = 0;
const SPAWN_ERROR_PREFIX: &str = "spawn-error: ";

/// What a bulk resume needs while it runs, after its request was answered.
#[derive(Clone)]
pub struct ResumeContext {
    pub agents: AgentGateway,
    pub store: Arc<dyn AgentStore>,
    /// The sweep results of `lost` sessions, for `resume.plan` (0017/D8).
    pub processes: Arc<SessionProcessRepository>,
    pub holds: Arc<HoldService>,
    pub events: EventHub,
    pub alerts: StorageAlerts,
    /// How long a start is watched for an end by itself (0013 A2).
    pub check_period: Duration,
}

/// The person action that a watch follows: the bulk resume of 0013 or the
/// wake of an asleep agent (0018/FR-007). Both start the same way and fall
/// back the same way; only the event and the hold reason name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Resume,
    Wake,
}

impl Flow {
    const fn first_request(self) -> &'static str {
        match self {
            Self::Resume => RESUME_REQUEST,
            Self::Wake => WAKE_REQUEST,
        }
    }

    const fn result_event(self) -> &'static str {
        match self {
            Self::Resume => RESUME_RESULT_EVENT,
            Self::Wake => WAKE_RESULT_EVENT,
        }
    }

    const fn failure_lead(self) -> &'static str {
        match self {
            Self::Resume => "the resume failed",
            Self::Wake => "the wake failed",
        }
    }
}

/// One `resume.result` or `wake.result` (resume and sleep protocol
/// contracts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeResult {
    Checking {
        continued: bool,
        reason: Option<String>,
    },
    Continued,
    NewSession {
        reason: Option<String>,
    },
    Held {
        reason: String,
    },
    Skipped {
        reason: String,
    },
}

impl ResumeResult {
    #[must_use]
    pub fn event(&self, flow: Flow, agent_id: AgentId) -> Event {
        let mut fields = Map::new();
        fields.insert("agent_id".to_owned(), json!(agent_id.to_string()));
        fields.insert("result".to_owned(), json!(self.name()));
        fields.insert(
            "is_final".to_owned(),
            json!(!matches!(self, Self::Checking { .. })),
        );
        if let Self::Checking { continued, .. } = self {
            fields.insert("continued".to_owned(), json!(continued));
        }
        if let Some(reason) = self.reason() {
            fields.insert("reason".to_owned(), json!(reason));
        }
        Event::new(flow.result_event(), fields)
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Checking { .. } => "checking",
            Self::Continued => "continued",
            Self::NewSession { .. } => "new_session",
            Self::Held { .. } => "held",
            Self::Skipped { .. } => "skipped",
        }
    }

    fn reason(&self) -> Option<&str> {
        match self {
            Self::Checking { reason, .. } | Self::NewSession { reason } => reason.as_deref(),
            Self::Held { reason } | Self::Skipped { reason } => Some(reason),
            Self::Continued => None,
        }
    }
}

/// One start of a bulk resume, read from the `agent.resume` answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    pub session_id: String,
    pub continued: bool,
    pub reason: Option<String>,
    /// The text of a start that never ran, from the session's exit result.
    pub did_not_start: Option<String>,
}

impl Started {
    /// `None` when the answer is not a session.
    #[must_use]
    pub fn from_answer(result: &Value) -> Option<Self> {
        let session_id = result.get("id")?.as_str()?.to_owned();
        let did_not_start = result
            .get("exit_result")
            .and_then(Value::as_str)
            .and_then(|text| text.strip_prefix(SPAWN_ERROR_PREFIX))
            .map(str::to_owned);
        Some(Self {
            session_id,
            continued: result.get("continued").and_then(Value::as_bool) == Some(true),
            reason: result
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_owned),
            did_not_start,
        })
    }

    fn checking(&self) -> ResumeResult {
        ResumeResult::Checking {
            continued: self.continued,
            reason: self.reason.clone(),
        }
    }

    /// The final result when the session ran through its check period, or
    /// the user acted on the agent meanwhile.
    fn settled(&self) -> ResumeResult {
        if self.continued {
            ResumeResult::Continued
        } else {
            ResumeResult::NewSession {
                reason: self.reason.clone(),
            }
        }
    }
}

/// How a watched session's check period ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEnd {
    /// The CLI ended by itself, or never started.
    ByItself { after: Duration },
    /// The user stopped the agent or the server stopped: not a refusal.
    ByOthers,
    /// The period passed with the session running, or the user started
    /// something else on the agent.
    StillRunning,
}

/// The end that an agent status means, or `None` for a running status.
#[must_use]
pub fn end_of(status: &str, after: Duration) -> Option<SessionEnd> {
    match status {
        "exited" | "failed" => Some(SessionEnd::ByItself { after }),
        "stopped" | "interrupted" => Some(SessionEnd::ByOthers),
        _ => None,
    }
}

/// Why the office held an agent whose new session failed too (0013/FR-005,
/// 0018/FR-007).
#[must_use]
pub fn hold_reason(flow: Flow, fresh: &Started, end: SessionEnd) -> String {
    let lead = flow.failure_lead();
    match (&fresh.did_not_start, end) {
        (Some(text), _) => format!("{lead}: the new session did not start: {text}"),
        (None, SessionEnd::ByItself { after }) => format!(
            "{lead}: the new session ended {} s after its start",
            after.as_secs()
        ),
        (None, _) => format!("{lead}: the new session ended"),
    }
}

/// Starts the agents one after another and watches each start on a task of
/// its own (0013 D3), so a slow check never delays the next start.
pub async fn run_resume(context: ResumeContext, order: Vec<AgentId>) {
    for agent_id in order {
        // Subscribed before the start, so no status of this start is missed.
        let statuses = context.events.subscribe();
        match context.first_start(Flow::Resume, agent_id).await {
            Ok(started) => {
                tokio::spawn(
                    context
                        .clone()
                        .watch(Flow::Resume, agent_id, statuses, started),
                );
            }
            Err(reason) => {
                context.publish(Flow::Resume, agent_id, &ResumeResult::Skipped { reason });
            }
        }
    }
}

impl ResumeContext {
    /// Wakes one asleep agent as the person (0018/FR-007): the first start
    /// answers the request, and its watch runs on a task of its own with the
    /// 0013 fallback and hold.
    ///
    /// # Errors
    /// The refusal of the first start, for example "the agent is no longer
    /// asleep".
    pub async fn wake(&self, agent_id: AgentId) -> Result<Value, (ErrorCode, String)> {
        // Subscribed before the start, so no status of this start is missed.
        let statuses = self.events.subscribe();
        let mut fields = Map::new();
        fields.insert("agent_id".to_owned(), json!(agent_id.to_string()));
        match self
            .ask(Flow::Wake.first_request(), fields, Actor::User)
            .await
        {
            Response::Success { result, .. } => {
                let started = Started::from_answer(&result).ok_or_else(|| {
                    (
                        ErrorCode::Internal,
                        "the wake answered without a session".to_owned(),
                    )
                })?;
                tokio::spawn(self.clone().watch(Flow::Wake, agent_id, statuses, started));
                Ok(result)
            }
            Response::Failure { code, message, .. } => Err((code, message)),
        }
    }

    /// The first start, as the user (0013/FR-003, 0018/FR-007).
    async fn first_start(&self, flow: Flow, agent_id: AgentId) -> Result<Started, String> {
        self.start(flow.first_request(), agent_id, None, Actor::User)
            .await
    }

    /// The one new session after a refused continue, as the office (FR-004).
    async fn fall_back(&self, agent_id: AgentId, refused: &Started) -> Result<Started, String> {
        self.start(
            RESUME_REQUEST,
            agent_id,
            Some(&refused.session_id),
            Actor::Office,
        )
        .await
    }

    async fn start(
        &self,
        kind: &str,
        agent_id: AgentId,
        after_session: Option<&str>,
        actor: Actor,
    ) -> Result<Started, String> {
        let mut fields = Map::new();
        fields.insert("agent_id".to_owned(), json!(agent_id.to_string()));
        if let Some(session_id) = after_session {
            fields.insert("after_session".to_owned(), json!(session_id));
        }
        match self.ask(kind, fields, actor).await {
            Response::Success { result, .. } => Started::from_answer(&result)
                .ok_or_else(|| "the start answered without a session".to_owned()),
            Response::Failure { message, .. } => Err(message),
        }
    }

    /// Runs `agent.resume` or `agent.wake` through the agent gateway, which
    /// keeps the status order, the stopping refusal, the events, and the
    /// action record.
    async fn ask(&self, kind: &str, fields: Map<String, Value>, actor: Actor) -> Response {
        let agents = self.agents.clone();
        let request = Request {
            id: OWN_REQUEST_ID,
            kind: kind.to_owned(),
            fields,
        };
        spawn_blocking(move || {
            let _scope = ActorScope::enter(actor);
            agents.run(&request)
        })
        .await
        .unwrap_or_else(|error| {
            Response::failure(
                OWN_REQUEST_ID,
                ErrorCode::Internal,
                format!("the start task failed: {error}"),
            )
        })
    }

    /// Watches the first start and, when the CLI refuses, the one new
    /// session after it (FR-004, FR-005). Never a third start (A8).
    async fn watch(
        self,
        flow: Flow,
        agent_id: AgentId,
        mut statuses: broadcast::Receiver<Arc<str>>,
        first: Started,
    ) {
        self.publish(flow, agent_id, &first.checking());
        let first_end = self.wait_for_end(&mut statuses, agent_id, &first).await;
        if !matches!(first_end, SessionEnd::ByItself { .. }) {
            return self.publish(flow, agent_id, &first.settled());
        }
        if !first.continued {
            return self
                .hold(flow, agent_id, hold_reason(flow, &first, first_end))
                .await;
        }
        // A new subscription: a status of the first start that is still
        // queued (a start that never ran ends without a wait) must not be
        // read as the end of the new session.
        let mut statuses = self.events.subscribe();
        let Ok(fresh) = self.fall_back(agent_id, &first).await else {
            // The user acted on the agent after the refusal; his action wins.
            return self.publish(flow, agent_id, &first.settled());
        };
        self.publish(flow, agent_id, &fresh.checking());
        let fresh_end = self.wait_for_end(&mut statuses, agent_id, &fresh).await;
        if matches!(fresh_end, SessionEnd::ByItself { .. }) {
            self.hold(flow, agent_id, hold_reason(flow, &fresh, fresh_end))
                .await;
        } else {
            self.publish(flow, agent_id, &fresh.settled());
        }
    }

    /// Waits for the agent's next ending status within the check period. The
    /// status bridge publishes it only after it stored the exit (D4).
    async fn wait_for_end(
        &self,
        statuses: &mut broadcast::Receiver<Arc<str>>,
        agent_id: AgentId,
        started: &Started,
    ) -> SessionEnd {
        let began = Instant::now();
        if started.did_not_start.is_some() {
            return SessionEnd::ByItself {
                after: Duration::ZERO,
            };
        }
        let deadline = began + self.check_period;
        loop {
            let status = match timeout_at(deadline, statuses.recv()).await {
                Err(_elapsed) => return SessionEnd::StillRunning,
                Ok(Ok(event)) => status_of(&event, agent_id),
                Ok(Err(RecvError::Lagged(_))) => self.stored_status(agent_id).await,
                Ok(Err(RecvError::Closed)) => return SessionEnd::ByOthers,
            };
            if let Some(end) = status.and_then(|status| end_of(&status, began.elapsed())) {
                return end;
            }
        }
    }

    /// The agent's status from the store, for when events were missed.
    async fn stored_status(&self, agent_id: AgentId) -> Option<String> {
        let store = Arc::clone(&self.store);
        spawn_blocking(move || store.find(agent_id))
            .await
            .ok()?
            .ok()?
            .map(|agent| agent.status.as_str().to_owned())
    }

    /// Puts the agent on hold with the reason (FR-005). An agent already on
    /// hold gets no second hold and no second notification (A9).
    async fn hold(&self, flow: Flow, agent_id: AgentId, reason: String) {
        let holds = Arc::clone(&self.holds);
        let stored_reason = reason.clone();
        let outcome = spawn_blocking(move || holds.hold_by_office(agent_id, &stored_reason)).await;
        let reason = match outcome {
            Ok(Ok(Some(_))) => reason,
            Ok(Ok(None)) => format!("{reason}; the agent was already on hold"),
            Ok(Err(error)) => self.hold_failed(&reason, &error.to_string()),
            Err(error) => self.hold_failed(&reason, &error.to_string()),
        };
        self.publish(flow, agent_id, &ResumeResult::Held { reason });
    }

    fn hold_failed(&self, reason: &str, error: &str) -> String {
        self.alerts.report_disk_failure(&format!(
            "the hold of a failed resume or wake could not be stored: {error}"
        ));
        format!("{reason}; the hold could not be stored: {error}")
    }

    fn publish(&self, flow: Flow, agent_id: AgentId, result: &ResumeResult) {
        self.events.publish(&result.event(flow, agent_id));
    }
}

/// The status of an `agent.status` event of this agent, or `None` for any
/// other event.
fn status_of(event: &str, agent_id: AgentId) -> Option<String> {
    let body: Value = serde_json::from_str(event).ok()?;
    if body.get("type")?.as_str()? != STATUS_EVENT
        || body.get("agent_id")?.as_str()? != agent_id.to_string()
    {
        return None;
    }
    Some(body.get("status")?.as_str()?.to_owned())
}

#[cfg(test)]
#[path = "resume_run.tests.rs"]
mod tests;
