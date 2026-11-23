// SPDX-License-Identifier: AGPL-3.0-or-later

//! The one place that ends a temp (0020 D7): its child temps first, then the
//! stop, the keep rule, the delete, the finished entry and the `end` record,
//! and the notice to the parent.

use std::collections::HashMap;
use std::sync::Arc;

use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::{WorktreeManager, WorktreeStatus};
use agent_domain::entities::{Agent, FinishedReport};
use agent_domain::enums::AgentStatus;
use hierarchy_application::TreeQuery;
use hierarchy_domain::{TreeRow, end_notice_line};
use kernel::{Actor, AgentId, visible_text};
use serde_json::{Map, Value, json};
use wire::{Request, Response};

use super::{END_CHECK_REQUEST, EndNote, KeptWork, NotificationBridge, TempHooks, TempSnapshot};
use crate::bootstrap::ServerLog;
use crate::server::AgentGateway;

/// Requests the server sends itself never reach a client, so their id is
/// never matched with an answer.
const OWN_REQUEST_ID: u32 = 0;

/// Why a temp ends, and what its finished entry and the end notice say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndReason {
    /// The office accepted a `done` report from the temp.
    Done {
        report: FinishedReport,
    },
    NoOutput {
        minutes: u32,
    },
    Exited {
        code: u32,
    },
    Failed {
        reason: String,
    },
    /// `by` is the whole actor phrase: "the person", an agent name, or "the
    /// office".
    Stopped {
        by: String,
    },
    EndedByPerson,
    ParentEnded {
        name: String,
    },
}

impl EndReason {
    /// The reason as the finished entry and the record show it.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Done { .. } => "done".to_owned(),
            Self::NoOutput { minutes } => format!("no output for {minutes} minutes"),
            Self::Exited { code } => format!("exited with code {code}"),
            Self::Failed { reason } => format!("failed: {reason}"),
            Self::Stopped { by } => format!("stopped by {by}"),
            Self::EndedByPerson => "ended by the person".to_owned(),
            Self::ParentEnded { name } => format!("its parent {name} ended"),
        }
    }

    /// The reasons whose end a running parent hears of (0020/FR-007).
    fn tells_parent(&self) -> bool {
        matches!(
            self,
            Self::NoOutput { .. } | Self::Exited { .. } | Self::Failed { .. }
        )
    }

    fn report(&self) -> Option<FinishedReport> {
        match self {
            Self::Done { report } => Some(report.clone()),
            _ => None,
        }
    }
}

/// One end to run.
#[derive(Debug, Clone)]
pub struct EndOrder {
    pub agent_id: AgentId,
    pub reason: EndReason,
    /// Who the stop and the `end` record name.
    pub actor: Actor,
    /// The status the watcher saw, when it chose a stopped, exited, or failed
    /// temp: the end goes on only while the temp still has it (D7 step 0).
    pub seen: Option<AgentStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndOutcome {
    Ended,
    /// Another end of the same temp runs.
    AlreadyEnding,
    /// The status changed since the watcher read it, so nothing ended.
    StatusChanged,
    NotATemp,
    Gone,
    Failed(String),
}

/// Ends temps. Clones share the `ending` set through the hooks.
#[derive(Clone)]
pub struct TempEnder {
    parts: Arc<EnderParts>,
}

/// What the ender works with. One shared allocation keeps the clones in the
/// server state and the tasks small.
struct EnderParts {
    gateway: AgentGateway,
    hooks: TempHooks,
    tree: Arc<TreeQuery>,
    worktrees: Arc<dyn WorktreeManager>,
    notifications: NotificationBridge,
    log: ServerLog,
}

impl TempEnder {
    #[must_use]
    pub fn new(
        gateway: AgentGateway,
        hooks: TempHooks,
        tree: Arc<TreeQuery>,
        worktrees: Arc<dyn WorktreeManager>,
        notifications: NotificationBridge,
        log: ServerLog,
    ) -> Self {
        Self {
            parts: Arc::new(EnderParts {
                gateway,
                hooks,
                tree,
                worktrees,
                notifications,
                log,
            }),
        }
    }

    /// Runs the end on the blocking thread it is called on. Every path
    /// leaves the `ending` set.
    #[must_use]
    pub fn end(&self, order: &EndOrder) -> EndOutcome {
        if let Err(outcome) = self.claim(order) {
            return outcome;
        }
        let outcome = self.end_claimed(order);
        self.parts.hooks.endings.release(order.agent_id);
        outcome
    }

    /// True while an end of the temp runs.
    #[must_use]
    pub fn is_ending(&self, agent_id: AgentId) -> bool {
        self.parts.hooks.endings.contains(agent_id)
    }

    fn claim(&self, order: &EndOrder) -> Result<(), EndOutcome> {
        let Some(seen) = order.seen else {
            return self.parts.hooks.endings.claim_temp(order.agent_id);
        };
        match self.checked_claim(order.agent_id, seen) {
            Ok(true) => Ok(()),
            Ok(false) => Err(EndOutcome::StatusChanged),
            Err(message) => Err(EndOutcome::Failed(message)),
        }
    }

    /// Step 0: the claim under the status order, so a new task or a
    /// continued session that holds the order finishes before the status is
    /// read.
    fn checked_claim(&self, agent_id: AgentId, seen: AgentStatus) -> Result<bool, String> {
        let request = office_request(
            END_CHECK_REQUEST,
            json!({"agent_id": agent_id.to_string(), "seen": seen.as_str()}),
        );
        match self.run_as(Actor::Office, &request) {
            Response::Success { result, .. } => {
                Ok(result.get("ending").and_then(Value::as_bool) == Some(true))
            }
            Response::Failure { message, .. } => Err(message),
        }
    }

    fn end_claimed(&self, order: &EndOrder) -> EndOutcome {
        let (agent, rows) = match self.load(order.agent_id) {
            Ok(loaded) => loaded,
            Err(outcome) => return outcome,
        };
        self.end_child_temps(&rows, &agent);
        if let Err(message) = self.stop_if_running(order) {
            return EndOutcome::Failed(message);
        }
        match self.remove(order, &rows) {
            Ok(()) => EndOutcome::Ended,
            Err(message) => {
                self.parts
                    .log
                    .error(&format!("the temp {} was not ended: {message}", agent.id));
                EndOutcome::Failed(message)
            }
        }
    }

    fn load(&self, agent_id: AgentId) -> Result<(Agent, Vec<TreeRow>), EndOutcome> {
        let agent = match self.parts.hooks.agents.find(agent_id) {
            Ok(Some(agent)) => agent,
            Ok(None) => return Err(EndOutcome::Gone),
            Err(error) => return Err(EndOutcome::Failed(error.to_string())),
        };
        if agent.temp.is_none() {
            return Err(EndOutcome::NotATemp);
        }
        let rows = self
            .parts
            .tree
            .store_snapshot()
            .map_err(|error| EndOutcome::Failed(error.to_string()))?;
        Ok((agent, rows))
    }

    /// Step 2: a child temp ends before its parent, with the parent named.
    fn end_child_temps(&self, rows: &[TreeRow], parent: &Agent) {
        let children = rows
            .iter()
            .filter(|row| row.parent_agent_id == Some(parent.id));
        for child in children {
            let order = EndOrder {
                agent_id: child.agent_id,
                reason: EndReason::ParentEnded {
                    name: parent.name.as_str().to_owned(),
                },
                actor: Actor::Office,
                seen: None,
            };
            if let EndOutcome::Failed(message) = self.end(&order) {
                self.parts.log.error(&format!(
                    "the child temp {} of {} was not ended: {message}",
                    child.agent_id, parent.id
                ));
            }
        }
    }

    /// Step 3: the stop of a running temp, as the actor the end names.
    fn stop_if_running(&self, order: &EndOrder) -> Result<(), String> {
        let running = self
            .parts
            .hooks
            .agents
            .find(order.agent_id)
            .map_err(|error| error.to_string())?
            .is_some_and(|agent| agent.status.is_running());
        if !running {
            return Ok(());
        }
        let request = office_request("agent.stop", agent_fields(order.agent_id));
        failure_text(&self.run_as(order.actor, &request))
    }

    /// Steps 4 to 7: the keep rule, the delete, the entry, the notice.
    fn remove(&self, order: &EndOrder, rows: &[TreeRow]) -> Result<(), String> {
        let agent = self
            .parts
            .hooks
            .agents
            .find(order.agent_id)
            .map_err(|error| error.to_string())?
            .ok_or("the temp is gone")?;
        let parent = parent_of(rows, agent.id);
        let snapshot = TempSnapshot::of(&agent, parent.as_ref().map(|(_, name)| name.clone()))
            .ok_or("the agent is not a temp")?;
        let choice = self.keep_rule(&agent);
        let removal_asked = choice.get("worktrees").and_then(Value::as_str) == Some("remove");
        let answer = self.run_as(Actor::Office, &office_request("agent.delete", choice));
        let outcome = match answer {
            Response::Success { result, .. } => result,
            Response::Failure { message, .. } => return Err(message),
        };
        let end = EndNote {
            reason: order.reason.text(),
            actor: order.actor,
            report: order.reason.report(),
        };
        let kept = KeptWork::from_outcome(&snapshot, &outcome, removal_asked);
        self.parts.hooks.ledger.record(&snapshot, &end, kept);
        self.tell_parent(&order.reason, &snapshot, parent.map(|(id, _)| id));
        Ok(())
    }

    /// Step 4: a worktree with no change and no commit of its own goes; any
    /// other stays (0010/FR-007 does the rest at the delete).
    fn keep_rule(&self, agent: &Agent) -> Value {
        let mut fields = agent_fields(agent.id);
        let remove_hash = agent
            .worktree
            .as_ref()
            .map(|worktree| self.parts.worktrees.status(worktree))
            .filter(holds_nothing)
            .map(|status| status.status_hash);
        match remove_hash {
            Some(hash) => {
                fields.insert("worktrees".to_owned(), json!("remove"));
                fields.insert(
                    "confirmed_status".to_owned(),
                    json!(HashMap::from([(agent.id.to_string(), hash)])),
                );
            }
            None => {
                fields.insert("worktrees".to_owned(), json!("keep"));
            }
        }
        Value::Object(fields)
    }

    /// Step 7 (FR-007): the end notice for a running parent; an idle end that
    /// no parent hears becomes one notification.
    fn tell_parent(&self, reason: &EndReason, snapshot: &TempSnapshot, parent: Option<AgentId>) {
        if !reason.tells_parent() {
            return;
        }
        let reason_text = reason.text();
        let line = end_notice_line(
            &visible_text(&snapshot.name),
            snapshot.agent_id,
            &visible_text(&reason_text),
        );
        let queued = parent
            .filter(|parent| self.parent_runs(*parent))
            .is_some_and(|parent| {
                self.parts
                    .hooks
                    .directives
                    .push_end_notice(snapshot.agent_id, parent, &line)
            });
        if !queued && matches!(reason, EndReason::NoOutput { .. }) {
            self.parts.notifications.record_temp_ended(
                snapshot.agent_id,
                &snapshot.name,
                &reason_text,
            );
        }
    }

    fn parent_runs(&self, parent: AgentId) -> bool {
        self.parts
            .hooks
            .agents
            .find(parent)
            .ok()
            .flatten()
            .is_some_and(|agent| agent.status.is_running())
    }

    fn run_as(&self, actor: Actor, request: &Request) -> Response {
        let _scope = ActorScope::enter(actor);
        self.parts.gateway.run(request)
    }
}

/// A worktree that holds no work: reachable, no changed file, and no commit
/// that its base lacks.
fn holds_nothing(status: &WorktreeStatus) -> bool {
    status.reachable && status.changed_files == 0 && status.own_commits == Some(0)
}

fn parent_of(rows: &[TreeRow], agent_id: AgentId) -> Option<(AgentId, String)> {
    let parent = rows
        .iter()
        .find(|row| row.agent_id == agent_id)?
        .parent_agent_id?;
    let row = rows.iter().find(|row| row.agent_id == parent)?;
    Some((parent, row.name.clone()))
}

fn agent_fields(agent_id: AgentId) -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert("agent_id".to_owned(), json!(agent_id.to_string()));
    fields
}

fn office_request(kind: &str, fields: impl Into<Value>) -> Request {
    let fields = match fields.into() {
        Value::Object(fields) => fields,
        _ => Map::new(),
    };
    Request {
        id: OWN_REQUEST_ID,
        kind: kind.to_owned(),
        fields,
    }
}

fn failure_text(response: &Response) -> Result<(), String> {
    match response {
        Response::Success { .. } => Ok(()),
        Response::Failure { message, .. } => Err(message.clone()),
    }
}

#[cfg(test)]
#[path = "temp_ender.tests.rs"]
mod tests;
