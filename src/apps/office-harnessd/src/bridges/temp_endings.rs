// SPDX-License-Identifier: AGPL-3.0-or-later

//! The temps that are being ended (0020 D7). While a temp is in the set, no
//! request may start it again, so no start slips in between the check that
//! chose to end it and its delete.

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use agent_application::common::interfaces::{AgentStore, SessionStore};
use agent_domain::value_objects::SessionId;
use kernel::AgentId;
use serde_json::{Value, json};
use wire::{ErrorCode, Request, Response};

use super::EndOutcome;

/// The office-only request that checks a status under the status order and
/// claims the temp for its end when the status is still the one the watcher
/// read.
pub const END_CHECK_REQUEST: &str = "temp.end_check";

/// What a request for an ending temp is told.
pub const ENDING_MESSAGE: &str = "the temp is ending";

/// The requests that would start the temp's CLI again.
const STARTING_REQUESTS: [&str; 5] = [
    "agent.start",
    "agent.new_task",
    "session.continue",
    "agent.resume",
    "agent.wake",
];

const SESSION_REQUEST: &str = "session.continue";

/// Held while a new temp is created and started. A new temp reads `stopped`
/// until its start takes the status order, so the watcher waits for every
/// guard before it judges a stopped temp.
#[must_use]
pub struct CreatingGuard(Arc<AtomicUsize>);

impl Drop for CreatingGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// The shared set of temps in their end. Clones share one set.
#[derive(Clone)]
pub struct TempEndings {
    ending: Arc<Mutex<HashSet<AgentId>>>,
    creating: Arc<AtomicUsize>,
    agents: Arc<dyn AgentStore>,
    sessions: Arc<dyn SessionStore>,
}

impl TempEndings {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>, sessions: Arc<dyn SessionStore>) -> Self {
        Self {
            ending: Arc::default(),
            creating: Arc::default(),
            agents,
            sessions,
        }
    }

    fn locked(&self) -> MutexGuard<'_, HashSet<AgentId>> {
        self.ending.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Joins the set. `false` when the temp is already in it.
    #[must_use]
    pub fn claim(&self, agent_id: AgentId) -> bool {
        self.locked().insert(agent_id)
    }

    /// Joins the set when the agent is a temp, so a `done` report of an agent
    /// that is not a temp never makes its start read "the temp is ending".
    ///
    /// # Errors
    ///
    /// The outcome that ends nothing: the agent is gone, is not a temp, or is
    /// already ending, or the store failed.
    pub fn claim_temp(&self, agent_id: AgentId) -> Result<(), EndOutcome> {
        match self.agents.find(agent_id) {
            Ok(Some(agent)) if agent.temp.is_some() => {}
            Ok(Some(_)) => return Err(EndOutcome::NotATemp),
            Ok(None) => return Err(EndOutcome::Gone),
            Err(error) => return Err(EndOutcome::Failed(error.to_string())),
        }
        if self.claim(agent_id) {
            Ok(())
        } else {
            Err(EndOutcome::AlreadyEnding)
        }
    }

    /// Marks a temp create as running until the guard drops.
    pub fn begin_create(&self) -> CreatingGuard {
        self.creating.fetch_add(1, Ordering::SeqCst);
        CreatingGuard(Arc::clone(&self.creating))
    }

    /// True while a temp is being created and started.
    #[must_use]
    pub fn is_creating(&self) -> bool {
        self.creating.load(Ordering::SeqCst) > 0
    }

    pub fn release(&self, agent_id: AgentId) {
        self.locked().remove(&agent_id);
    }

    #[must_use]
    pub fn contains(&self, agent_id: AgentId) -> bool {
        self.locked().contains(&agent_id)
    }

    /// True when `request` would start a temp that is ending.
    #[must_use]
    pub fn refuses(&self, request: &Request) -> bool {
        if !STARTING_REQUESTS.contains(&request.kind.as_str()) || self.locked().is_empty() {
            return false;
        }
        self.target_of(request)
            .is_some_and(|agent_id| self.contains(agent_id))
    }

    /// The agent a starting request names: an agent id, or the agent of the
    /// session that a continue names.
    fn target_of(&self, request: &Request) -> Option<AgentId> {
        let text = |name: &str| request.fields.get(name).and_then(Value::as_str);
        if request.kind == SESSION_REQUEST {
            let session_id = SessionId::from_str(text("session_id")?).ok()?;
            let session = self.sessions.find(session_id).ok()??;
            return Some(session.agent_id);
        }
        AgentId::from_str(text("agent_id")?).ok()
    }

    /// Answers `temp.end_check`: claims the temp when it still is a temp with
    /// the status the watcher saw. The caller holds the status order, so no
    /// new task or continued session changes the status in between.
    #[must_use]
    pub fn end_check(&self, request: &Request) -> Response {
        let seen = request.fields.get("seen").and_then(Value::as_str);
        let agent_id = request
            .fields
            .get("agent_id")
            .and_then(Value::as_str)
            .and_then(|text| AgentId::from_str(text).ok());
        let (Some(agent_id), Some(seen)) = (agent_id, seen) else {
            return Response::failure(
                request.id,
                ErrorCode::Invalid,
                "temp.end_check needs agent_id and seen",
            );
        };
        match self.claim_if_unchanged(agent_id, seen) {
            Ok(claimed) => Response::success(request.id, json!({"ending": claimed})),
            Err(message) => Response::failure(request.id, ErrorCode::Internal, message),
        }
    }

    fn claim_if_unchanged(&self, agent_id: AgentId, seen: &str) -> Result<bool, String> {
        let agent = self
            .agents
            .find(agent_id)
            .map_err(|error| error.to_string())?;
        let unchanged =
            agent.is_some_and(|agent| agent.temp.is_some() && agent.status.as_str() == seen);
        Ok(unchanged && self.claim(agent_id))
    }
}

#[cfg(test)]
#[path = "temp_endings.tests.rs"]
mod tests;
