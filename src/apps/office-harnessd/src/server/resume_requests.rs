// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

use agent_domain::enums::AgentStatus;
use hierarchy_application::queries::tree::TreeQuery;
use hierarchy_domain::HoldScope;
use kernel::AgentId;
use serde_json::{Map, Value, json};
use tokio::task::spawn_blocking;
use wire::{ErrorCode, Request, Response};

use super::agent_requests::STOPPING_MESSAGE;
use super::resume_order::start_order;
use super::resume_run::{ResumeContext, run_resume};
use crate::sweep::{SweepGate, SweepState};

const RESUME_PLAN: &str = "resume.plan";
const RESUME_START: &str = "resume.start";
const SWEEP_STATE: &str = "sweep.state";
const RESUME_REQUESTS: [&str; 3] = [RESUME_PLAN, RESUME_START, SWEEP_STATE];
const AGENT_IDS_FIELD: &str = "agent_ids";
const MAX_AGENT_IDS: usize = 1000;
const SESSION_LIST: &str = "session.list";
const LOST_EXIT_RESULT: &str = "lost";
const NO_EARLIER_SESSION: &str = "the agent has no earlier session";

type Refusal = (ErrorCode, String);

/// Runs `resume.plan` and `resume.start` (0013 D2) for the user's
/// connections only: the agent connection answers neither (A10). The
/// starts run on a task of their own after the answer (D3), and each agent's
/// progress reaches every client as `resume.result`.
#[derive(Clone)]
pub struct ResumeGateway {
    context: ResumeContext,
    tree: Arc<TreeQuery>,
    /// The sweep of 0017: `resume.start` waits for it, and `resume.plan`
    /// answers that it runs.
    sweep_gate: SweepGate,
}

impl ResumeGateway {
    #[must_use]
    pub fn new(context: ResumeContext, tree: Arc<TreeQuery>) -> Self {
        Self {
            context,
            tree,
            sweep_gate: SweepGate::default(),
        }
    }

    #[must_use]
    pub fn with_sweep_gate(mut self, sweep_gate: SweepGate) -> Self {
        self.sweep_gate = sweep_gate;
        self
    }

    #[must_use]
    pub fn accepts(kind: &str) -> bool {
        RESUME_REQUESTS.contains(&kind)
    }

    pub async fn handle(&self, request: Request) -> Response {
        let id = request.id;
        if request.kind == RESUME_START && self.sweep_gate.wait_until_done().await.is_err() {
            return Response::failure(id, ErrorCode::Refused, STOPPING_MESSAGE);
        }
        let gateway = self.clone();
        let answered = spawn_blocking(move || gateway.answer(&request)).await;
        match answered {
            Ok(Ok((result, order))) => {
                if !order.is_empty() {
                    tokio::spawn(run_resume(self.context.clone(), order));
                }
                Response::success(id, result)
            }
            Ok(Err((code, message))) => Response::failure(id, code, message),
            Err(error) => Response::failure(
                id,
                ErrorCode::Internal,
                format!("the request task failed: {error}"),
            ),
        }
    }

    /// The answer and, for `resume.start`, the agents to start in order.
    fn answer(&self, request: &Request) -> Result<(Value, Vec<AgentId>), Refusal> {
        refuse_unknown_fields(request)?;
        if request.kind == SWEEP_STATE {
            return Ok((
                json!({ "state": self.sweep_gate.state().as_str() }),
                Vec::new(),
            ));
        }
        if request.kind == RESUME_PLAN {
            return Ok((self.plan()?, Vec::new()));
        }
        let order = self.ordered(&agent_ids_of(request)?)?;
        let listed: Vec<String> = order.iter().map(AgentId::to_string).collect();
        Ok((json!({ "order": listed }), order))
    }

    /// The interrupted agents in start order, with what will happen to each
    /// (FR-002, FR-008).
    fn plan(&self) -> Result<Value, Refusal> {
        if self.sweep_gate.state() == SweepState::Running {
            return Ok(json!({ "sweeping": true, "agents": [] }));
        }
        let interrupted: Vec<AgentId> = self
            .context
            .store
            .list()
            .map_err(|error| internal(&error))?
            .into_iter()
            .filter(|agent| agent.status == AgentStatus::Interrupted)
            .map(|agent| agent.id)
            .collect();
        let held = self.held_agents();
        let entries = self
            .ordered(&interrupted)?
            .into_iter()
            .map(|agent_id| self.plan_entry(agent_id, held.contains(&agent_id)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({ "sweeping": false, "agents": entries }))
    }

    fn plan_entry(&self, agent_id: AgentId, held: bool) -> Result<Value, Refusal> {
        let last_session = self.last_session(agent_id)?;
        let can_continue = last_session
            .as_ref()
            .and_then(|session| session.get("can_continue"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let reason = match &last_session {
            None => Some(NO_EARLIER_SESSION.to_owned()),
            Some(session) => session
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_owned),
        };
        let after_unclean_stop = last_session
            .as_ref()
            .and_then(|session| session.get("exit_result"))
            .and_then(Value::as_str)
            == Some(LOST_EXIT_RESULT);
        let mut entry = json!({
            "agent_id": agent_id.to_string(),
            "last_session": last_session,
            "can_continue": can_continue,
            "held": held,
            "after_unclean_stop": after_unclean_stop,
        });
        if let Some(reason) = reason {
            entry["reason"] = json!(reason);
        }
        Ok(entry)
    }

    /// The newest `session.list` entry, which carries `can_continue` and the
    /// reason the agent module computes for it.
    fn last_session(&self, agent_id: AgentId) -> Result<Option<Value>, Refusal> {
        let mut fields = Map::new();
        fields.insert("agent_id".to_owned(), json!(agent_id.to_string()));
        let request = Request {
            id: 0,
            kind: SESSION_LIST.to_owned(),
            fields,
        };
        match self.context.agents.run(&request) {
            Response::Success { result, .. } => Ok(result
                .as_array()
                .and_then(|sessions| sessions.first())
                .cloned()),
            Response::Failure { code, message, .. } => Err((code, message)),
        }
    }

    fn held_agents(&self) -> HashSet<AgentId> {
        self.context
            .holds
            .list()
            .agents
            .iter()
            .filter_map(|hold| match hold.scope {
                HoldScope::Agent(agent_id) => Some(agent_id),
                HoldScope::Office => None,
            })
            .collect()
    }

    /// The agents in start order (D5). Ids that the store does not know keep
    /// their place at the end; their start answers `not_found` and they are
    /// skipped.
    fn ordered(&self, agent_ids: &[AgentId]) -> Result<Vec<AgentId>, Refusal> {
        let wanted: HashSet<AgentId> = agent_ids.iter().copied().collect();
        let listed: Vec<AgentId> = self
            .context
            .store
            .list()
            .map_err(|error| internal(&error))?
            .into_iter()
            .map(|agent| agent.id)
            .filter(|agent_id| wanted.contains(agent_id))
            .collect();
        let places = self.tree.row_places().map_err(|error| internal(&error))?;
        let mut order = start_order(&listed, &places);
        let known: HashSet<AgentId> = listed.into_iter().collect();
        order.extend(
            agent_ids
                .iter()
                .filter(|agent_id| !known.contains(agent_id)),
        );
        Ok(order)
    }
}

/// 1 to `MAX_AGENT_IDS` distinct agent ids.
fn agent_ids_of(request: &Request) -> Result<Vec<AgentId>, Refusal> {
    let invalid = |message: &str| (ErrorCode::Invalid, message.to_owned());
    let values = request
        .fields
        .get(AGENT_IDS_FIELD)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("agent_ids must be a list of agent ids"))?;
    if values.is_empty() || values.len() > MAX_AGENT_IDS {
        return Err(invalid("agent_ids must hold 1 to 1000 agent ids"));
    }
    let mut seen = HashSet::new();
    values
        .iter()
        .map(|value| {
            let agent_id = value
                .as_str()
                .and_then(|text| AgentId::from_str(text).ok())
                .ok_or_else(|| invalid("agent_ids holds a value that is not an agent id"))?;
            if seen.insert(agent_id) {
                Ok(agent_id)
            } else {
                Err(invalid("agent_ids holds the same agent twice"))
            }
        })
        .collect()
}

/// `invalid` for a field the request does not take.
fn refuse_unknown_fields(request: &Request) -> Result<(), Refusal> {
    let known: &[&str] = if request.kind == RESUME_START {
        &[AGENT_IDS_FIELD]
    } else {
        &[]
    };
    match request
        .fields
        .keys()
        .find(|field| !known.contains(&field.as_str()))
    {
        Some(field) => Err((
            ErrorCode::Invalid,
            format!("{field} is not a field of {}", request.kind),
        )),
        None => Ok(()),
    }
}

fn internal(error: &impl ToString) -> Refusal {
    (ErrorCode::Internal, error.to_string())
}

#[cfg(test)]
#[path = "resume_requests.tests.rs"]
mod tests;
