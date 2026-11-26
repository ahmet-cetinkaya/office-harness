// SPDX-License-Identifier: AGPL-3.0-or-later

//! The user's `temp.*` requests (0020 D11): the list of temps and finished
//! entries, the end of a temp, and the dismiss of a finished entry. The agent
//! connection never reaches this gateway.

use std::str::FromStr;
use std::sync::Arc;

use agent_application::common::interfaces::{AgentStore, Clock, FinishedTempStore};
use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{FinishedTempId, creator_text};
use agent_presentation::contracts::{Fields, TempDismissRequest, TempEndRequest};
use agent_presentation::mappings::{agent_json, finished_temp_json};
use hierarchy_application::TreeQuery;
use hierarchy_domain::TreeRow;
use kernel::{Actor, AgentId};
use serde_json::{Value, json};
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::agent_requests::fill_places;
use super::temp_run::QuietBoard;
use crate::bootstrap::StorageAlerts;
use crate::bridges::{
    END_CHECK_REQUEST, EndOrder, EndOutcome, EndReason, SystemClock, TempEnder, TempLedger,
};

const PREFIX: &str = "temp.";
const LIST_REQUEST: &str = "temp.list";
const END_REQUEST: &str = "temp.end";
const DISMISS_REQUEST: &str = "temp.dismiss";

const OFFICE_ONLY: &str = "temp.end_check is made by the office only";
const NOT_A_TEMP: &str = "the agent is not a temp";
const ALREADY_ENDED: &str = "the temp already ended";
const THE_PERSON: &str = "the person";
const THE_OFFICE: &str = "the office";
const MILLIS_PER_SECOND: i64 = 1000;

/// Runs the `temp.*` requests of a user connection. The use cases block, so
/// each call runs on a blocking thread.
#[derive(Clone)]
pub struct TempGateway {
    ender: TempEnder,
    agents: Arc<dyn AgentStore>,
    finished: Arc<dyn FinishedTempStore>,
    ledger: Arc<TempLedger>,
    tree: Arc<TreeQuery>,
    board: QuietBoard,
    alerts: StorageAlerts,
}

impl TempGateway {
    #[must_use]
    pub fn new(
        ender: TempEnder,
        agents: Arc<dyn AgentStore>,
        finished: Arc<dyn FinishedTempStore>,
        ledger: Arc<TempLedger>,
        tree: Arc<TreeQuery>,
        board: QuietBoard,
        alerts: StorageAlerts,
    ) -> Self {
        Self {
            ender,
            agents,
            finished,
            ledger,
            tree,
            board,
            alerts,
        }
    }

    /// True for every request of the `temp.` family.
    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        kind.starts_with(PREFIX)
    }

    /// The ender, for the done hook of the agent connection.
    #[must_use]
    pub fn ender(&self) -> &TempEnder {
        &self.ender
    }

    /// True when the agent exists and is a temp.
    #[must_use]
    pub fn is_temp(&self, agent_id: AgentId) -> bool {
        self.agents
            .find(agent_id)
            .ok()
            .flatten()
            .is_some_and(|agent| agent.temp.is_some())
    }

    pub async fn handle(&self, request: Request) -> Response {
        let id = request.id;
        let gateway = self.clone();
        spawn_blocking(move || gateway.run(&request))
            .await
            .unwrap_or_else(|error| {
                Response::failure(
                    id,
                    ErrorCode::Internal,
                    format!("the request task failed: {error}"),
                )
            })
    }

    fn run(&self, request: &Request) -> Response {
        let answer = match request.kind.as_str() {
            LIST_REQUEST => self.list(request),
            END_REQUEST => self.end(request),
            DISMISS_REQUEST => self.dismiss(request),
            END_CHECK_REQUEST => Err((ErrorCode::Invalid, OFFICE_ONLY.to_owned())),
            other => Err((
                ErrorCode::NotFound,
                format!("no request of the type {other}"),
            )),
        };
        match answer {
            Ok(result) => Response::success(request.id, result),
            Err((code, message)) => Response::failure(request.id, code, message),
        }
    }

    fn list(&self, request: &Request) -> Answer {
        Fields(&request.fields)
            .refuse_other_than(LIST_REQUEST, &[])
            .map_err(invalid)?;
        let agents = self.agents.list().map_err(|error| internal(&error))?;
        let rows = self
            .tree
            .store_snapshot()
            .map_err(|error| internal(&error))?;
        let places = self.tree.row_places().map_err(|error| internal(&error))?;
        let temps: Vec<Value> = agents
            .iter()
            .filter(|agent| agent.temp.is_some())
            .map(|agent| {
                let mut entry = self.temp_entry(agent, &agents, &rows);
                if let Some(object) = entry.get_mut("agent") {
                    fill_places(object, &places);
                }
                entry
            })
            .collect();
        let finished: Vec<Value> = self
            .finished
            .list()
            .map_err(|error| internal(&error))?
            .iter()
            .map(|entry| {
                let mut row = finished_temp_json(entry);
                row["creator_name"] = json!(
                    creator_name(entry.creator, &agents)
                        .unwrap_or_else(|| creator_text(entry.creator))
                );
                row
            })
            .collect();
        Ok(json!({"temps": temps, "finished": finished}))
    }

    fn temp_entry(&self, agent: &Agent, agents: &[Agent], rows: &[TreeRow]) -> Value {
        let creator = agent.temp.as_ref().map(|temp| temp.creator);
        let parent_name = rows
            .iter()
            .find(|row| row.agent_id == agent.id)
            .and_then(|row| row.parent_agent_id)
            .and_then(|parent| agents.iter().find(|other| other.id == parent))
            .map(|parent| parent.name.as_str().to_owned());
        let running = agent.status.is_running() || agent.status == AgentStatus::Asleep;
        let quiet_seconds = self
            .board
            .quiet_for(agent.id)
            .filter(|_| running)
            .map(|quiet| quiet.as_secs());
        json!({
            "agent": agent_json(agent),
            "creator_name": creator.and_then(|creator| creator_name(creator, agents)),
            "parent_name": parent_name,
            "run_seconds": run_seconds(agent),
            "quiet_seconds": quiet_seconds,
        })
    }

    fn end(&self, request: &Request) -> Answer {
        let body = TempEndRequest::read(&request.fields).map_err(invalid)?;
        let agent_id = AgentId::from_str(&body.agent_id).map_err(|_| {
            invalid(format!(
                "agent_id is not a valid identifier: {}",
                body.agent_id
            ))
        })?;
        self.require_temp(agent_id)?;
        let order = EndOrder {
            agent_id,
            reason: EndReason::EndedByPerson,
            actor: Actor::User,
            seen: None,
        };
        match self.ender.end(&order) {
            EndOutcome::Ended => Ok(json!({})),
            EndOutcome::NotATemp => Err((ErrorCode::Invalid, NOT_A_TEMP.to_owned())),
            EndOutcome::Failed(message) => Err((ErrorCode::Internal, message)),
            EndOutcome::AlreadyEnding | EndOutcome::Gone | EndOutcome::StatusChanged => {
                Err((ErrorCode::Conflict, ALREADY_ENDED.to_owned()))
            }
        }
    }

    /// An agent that is gone answers `conflict` when a finished entry names
    /// it, and `not_found` when none ever did.
    fn require_temp(&self, agent_id: AgentId) -> Result<(), (ErrorCode, String)> {
        match self
            .agents
            .find(agent_id)
            .map_err(|error| internal(&error))?
        {
            Some(agent) if agent.temp.is_some() => Ok(()),
            Some(_) => Err((ErrorCode::Invalid, NOT_A_TEMP.to_owned())),
            None => {
                let ended = self
                    .finished
                    .list()
                    .map_err(|error| internal(&error))?
                    .iter()
                    .any(|entry| entry.agent_id == agent_id);
                Err(if ended {
                    (ErrorCode::Conflict, ALREADY_ENDED.to_owned())
                } else {
                    (ErrorCode::NotFound, format!("no agent {agent_id}"))
                })
            }
        }
    }

    fn dismiss(&self, request: &Request) -> Answer {
        let body = TempDismissRequest::read(&request.fields).map_err(invalid)?;
        let entry_id = FinishedTempId::from_str(&body.entry_id).map_err(|_| {
            invalid(format!(
                "entry_id is not a valid identifier: {}",
                body.entry_id
            ))
        })?;
        let removed = self.ledger.dismiss(entry_id).map_err(|error| {
            self.alerts.report_disk_failure(&error.to_string());
            internal(&error)
        })?;
        if !removed {
            return Err((ErrorCode::NotFound, format!("no finished entry {entry_id}")));
        }
        Ok(json!({}))
    }
}

type Answer = Result<Value, (ErrorCode, String)>;

fn invalid(message: impl Into<String>) -> (ErrorCode, String) {
    (ErrorCode::Invalid, message.into())
}

fn internal(error: &impl ToString) -> (ErrorCode, String) {
    (ErrorCode::Internal, error.to_string())
}

/// "the person", the creating agent's name, or `None` when that agent is gone.
fn creator_name(creator: Actor, agents: &[Agent]) -> Option<String> {
    match creator {
        Actor::User => Some(THE_PERSON.to_owned()),
        Actor::Office => Some(THE_OFFICE.to_owned()),
        Actor::Agent(agent_id) => agents
            .iter()
            .find(|agent| agent.id == agent_id)
            .map(|agent| agent.name.as_str().to_owned()),
        Actor::Schedule(schedule_id) => Some(format!("schedule {schedule_id}")),
    }
}

fn run_seconds(agent: &Agent) -> i64 {
    let elapsed = SystemClock.now().as_unix_millis() - agent.created_at.as_unix_millis();
    elapsed.max(0) / MILLIS_PER_SECOND
}
