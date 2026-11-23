// SPDX-License-Identifier: AGPL-3.0-or-later

//! What the agent gateway does around a request because of temps (0020): it
//! refuses the start of a temp that ends, answers the end check, queues the
//! objective of a new temp, and keeps a finished entry for each temp that the
//! person deletes.

use std::sync::Arc;

use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::AgentStore;
use hierarchy_application::commands::send_directive::SendDirectiveHandler;
use hierarchy_domain::{TreeRow, descendants_of};
use kernel::{Actor, AgentId};
use serde_json::Value;
use wire::{Request, Response};

use super::{EndNote, KeptWork, TempEndings, TempLedger, TempSnapshot};

const DELETED_BY_PERSON: &str = "deleted by the person";
const REMOVE_CHOICE: &str = "remove";

#[derive(Clone)]
pub struct TempHooks {
    pub endings: TempEndings,
    pub ledger: TempLedger,
    pub directives: Arc<SendDirectiveHandler>,
    pub agents: Arc<dyn AgentStore>,
}

impl TempHooks {
    /// The temps among the agents that a delete by the person removes: the
    /// named agent, and its descendants when the request asks for them. Read
    /// before the delete, because the agents are gone after it.
    #[must_use]
    pub fn before_delete(&self, rows: &[TreeRow], request: &Request) -> Vec<TempSnapshot> {
        if ActorScope::current() != Actor::User {
            return Vec::new();
        }
        let Some(named) = agent_of(request) else {
            return Vec::new();
        };
        let mut affected = vec![named];
        if includes_descendants(request) {
            affected.extend(descendants_of(rows, named));
        }
        affected
            .into_iter()
            .filter_map(|agent_id| self.snapshot_of(rows, agent_id))
            .collect()
    }

    fn snapshot_of(&self, rows: &[TreeRow], agent_id: AgentId) -> Option<TempSnapshot> {
        let agent = self.agents.find(agent_id).ok()??;
        let parent_name = rows
            .iter()
            .find(|row| row.agent_id == agent_id)
            .and_then(|row| row.parent_agent_id)
            .and_then(|parent| rows.iter().find(|row| row.agent_id == parent))
            .map(|row| row.name.clone());
        TempSnapshot::of(&agent, parent_name)
    }

    /// Keeps one finished entry for each snapshot whose agent is gone now:
    /// all of them after a delete that worked, and the ones a failed subtree
    /// delete had already removed.
    pub fn after_delete(&self, snapshots: &[TempSnapshot], request: &Request, response: &Response) {
        let named = agent_of(request);
        let removal_asked =
            request.fields.get("worktrees").and_then(Value::as_str) == Some(REMOVE_CHOICE);
        for snapshot in snapshots {
            if !matches!(self.agents.find(snapshot.agent_id), Ok(None)) {
                continue;
            }
            let kept = match response {
                Response::Success { result, .. } => {
                    KeptWork::from_outcome(snapshot, result, removal_asked)
                }
                Response::Failure { .. } => KeptWork::default(),
            };
            let reason = if Some(snapshot.agent_id) == named {
                DELETED_BY_PERSON.to_owned()
            } else {
                format!(
                    "its parent {} was deleted",
                    snapshot.parent_name.as_deref().unwrap_or_default()
                )
            };
            let end = EndNote {
                reason,
                actor: Actor::User,
                report: None,
            };
            self.ledger.record(snapshot, &end, kept);
        }
    }
}

fn agent_of(request: &Request) -> Option<AgentId> {
    request
        .fields
        .get("agent_id")
        .and_then(Value::as_str)
        .and_then(AgentId::from_uuid_text)
}

fn includes_descendants(request: &Request) -> bool {
    request
        .fields
        .get("include_descendants")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}
