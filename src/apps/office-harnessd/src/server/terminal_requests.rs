// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use kernel::AgentId;
use serde::de::DeserializeOwned;
use serde_json::Value;
use terminal_application::common::models::SubscriptionId;
use terminal_presentation::contracts::{AttachRequest, DetachRequest, InputRequest, ResizeRequest};
use terminal_presentation::handlers::{ErrorKind, HandlerError, TerminalHandlers};
use tokio::sync::{Notify, mpsc};
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::channel_subscriber::ChannelSubscriber;
use crate::bridges::{PersonInput, SavedOutputs};

/// The host sends every request whose type starts with this to the module.
pub const TERMINAL_PREFIX: &str = "terminal.";

const ATTACH: &str = "terminal.attach";
const DETACH: &str = "terminal.detach";
const INPUT: &str = "terminal.input";
const RESIZE: &str = "terminal.resize";

/// How many output frames one connection may have waiting before the server
/// drops it from the terminal.
pub const FRAME_BACKLOG: usize = 1024;

/// What one connection is attached to, and the channel that carries its live
/// frames.
pub struct ConnectionTerminals {
    subscriptions: HashMap<AgentId, SubscriptionId>,
    frames: mpsc::Sender<Vec<u8>>,
    overflow: Arc<Notify>,
}

impl ConnectionTerminals {
    #[must_use]
    pub fn new(frames: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            subscriptions: HashMap::new(),
            frames,
            overflow: Arc::new(Notify::new()),
        }
    }

    /// Signalled when a terminal dropped this connection because its frames
    /// piled up. The connection must close so the client attaches again.
    #[must_use]
    pub fn overflow_signal(&self) -> Arc<Notify> {
        Arc::clone(&self.overflow)
    }
}

/// The answer to a terminal request. `snapshot_frame` is sent right after the
/// answer, before any live frame.
pub struct TerminalAnswer {
    pub response: Response,
    pub snapshot_frame: Option<Vec<u8>>,
}

impl TerminalAnswer {
    fn done(response: Response) -> Self {
        Self {
            response,
            snapshot_frame: None,
        }
    }
}

/// Runs the `terminal.*` requests. The handlers block, so each call runs on a
/// blocking thread.
pub struct TerminalGateway {
    handlers: Arc<TerminalHandlers>,
    saved_outputs: SavedOutputs,
    /// Stamped by every person input, so directives wait while a person
    /// types (0012/FR-002).
    person_input: PersonInput,
}

impl TerminalGateway {
    #[must_use]
    pub fn new(
        handlers: Arc<TerminalHandlers>,
        saved_outputs: SavedOutputs,
        person_input: PersonInput,
    ) -> Self {
        Self {
            handlers,
            saved_outputs,
            person_input,
        }
    }

    pub async fn handle(
        &self,
        request: &Request,
        connection: &mut ConnectionTerminals,
    ) -> TerminalAnswer {
        let id = request.id;
        let outcome = match request.kind.as_str() {
            ATTACH => self.attach(request, connection).await,
            DETACH => self.detach(request, connection).await,
            INPUT => self.input(request).await.map(TerminalAnswer::done),
            RESIZE => self.resize(request).await.map(TerminalAnswer::done),
            other => {
                return TerminalAnswer::done(Response::failure(
                    id,
                    ErrorCode::NotFound,
                    format!("no request of the type {other}"),
                ));
            }
        };
        outcome.unwrap_or_else(|error| TerminalAnswer::done(failure(id, &error)))
    }

    /// Detaches everything the closed connection was attached to.
    pub async fn release(&self, connection: ConnectionTerminals) {
        for (agent_id, subscription) in connection.subscriptions {
            self.detach_agent(agent_id, subscription).await;
        }
    }

    async fn attach(
        &self,
        request: &Request,
        connection: &mut ConnectionTerminals,
    ) -> Result<TerminalAnswer, HandlerError> {
        let body: AttachRequest = parse_body(request)?;
        let agent_id = parse_agent_id(&body.agent_id)?;
        if let Some(previous) = connection.subscriptions.remove(&agent_id) {
            self.detach_agent(agent_id, previous).await;
        }
        let subscriber = Arc::new(ChannelSubscriber::new(
            agent_id,
            connection.frames.clone(),
            Arc::clone(&connection.overflow),
        ));
        let handlers = Arc::clone(&self.handlers);
        let saved_outputs = self.saved_outputs.clone();
        let attached = run_blocking(move || {
            // The output of the agent's last session, shown when it does not run.
            let saved_output = saved_outputs
                .last_session_output(agent_id)
                .map_err(|error| HandlerError {
                    kind: ErrorKind::Internal,
                    message: error.to_string(),
                })?;
            handlers.attach(&body, saved_output.as_ref(), subscriber)
        })
        .await?;
        if let Some(subscription) = attached.subscription {
            connection.subscriptions.insert(agent_id, subscription);
        }
        Ok(TerminalAnswer {
            response: Response::success(request.id, Value::Null),
            snapshot_frame: Some(attached.snapshot_frame),
        })
    }

    async fn detach(
        &self,
        request: &Request,
        connection: &mut ConnectionTerminals,
    ) -> Result<TerminalAnswer, HandlerError> {
        let body: DetachRequest = parse_body(request)?;
        let agent_id = parse_agent_id(&body.agent_id)?;
        if let Some(subscription) = connection.subscriptions.remove(&agent_id) {
            self.detach_agent(agent_id, subscription).await;
        }
        Ok(TerminalAnswer::done(Response::success(
            request.id,
            Value::Null,
        )))
    }

    async fn input(&self, request: &Request) -> Result<Response, HandlerError> {
        let body: InputRequest = parse_body(request)?;
        let handlers = Arc::clone(&self.handlers);
        let person_input = self.person_input.clone();
        run_blocking(move || {
            // Before the bytes go in, so a directive gate never reads a
            // terminal as untouched while a person's keys are on their way.
            // On the blocking pool, because the wait end may read and write
            // the store (0022). The handler refuses an id or data that does
            // not parse.
            if let (Ok(agent_id), Ok(bytes)) = (body.agent_id.parse::<AgentId>(), body.bytes()) {
                person_input.stamp(agent_id, &bytes);
            }
            handlers.input(&body)
        })
        .await?;
        Ok(Response::success(request.id, Value::Null))
    }

    async fn resize(&self, request: &Request) -> Result<Response, HandlerError> {
        let body: ResizeRequest = parse_body(request)?;
        let handlers = Arc::clone(&self.handlers);
        run_blocking(move || handlers.resize(&body)).await?;
        Ok(Response::success(request.id, Value::Null))
    }

    async fn detach_agent(&self, agent_id: AgentId, subscription: SubscriptionId) {
        let handlers = Arc::clone(&self.handlers);
        let body = DetachRequest {
            agent_id: agent_id.to_string(),
        };
        // A detach that fails means the terminal is gone, which is the goal.
        let _ = run_blocking(move || handlers.detach(&body, subscription)).await;
    }
}

fn parse_body<T: DeserializeOwned>(request: &Request) -> Result<T, HandlerError> {
    serde_json::from_value(Value::Object(request.fields.clone())).map_err(|error| HandlerError {
        kind: ErrorKind::Invalid,
        message: error.to_string(),
    })
}

fn parse_agent_id(text: &str) -> Result<AgentId, HandlerError> {
    text.parse().map_err(|_| HandlerError {
        kind: ErrorKind::Invalid,
        message: format!("agent_id is not a valid identifier: {text}"),
    })
}

async fn run_blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, HandlerError> + Send + 'static,
) -> Result<T, HandlerError> {
    spawn_blocking(work).await.unwrap_or_else(|error| {
        Err(HandlerError {
            kind: ErrorKind::Internal,
            message: format!("the terminal task failed: {error}"),
        })
    })
}

fn failure(id: u32, error: &HandlerError) -> Response {
    let code = match error.kind {
        ErrorKind::Invalid => ErrorCode::Invalid,
        ErrorKind::Conflict => ErrorCode::Conflict,
        ErrorKind::NotFound => ErrorCode::NotFound,
        ErrorKind::Internal => ErrorCode::Internal,
    };
    Response::failure(id, code, error.message.clone())
}
