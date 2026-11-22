// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use hierarchy_application::HierarchyApplicationError;
use hierarchy_application::commands::subtree::SubtreeHandler;
use hierarchy_application::common::interfaces::{HierarchyStore, TreeExclusion, TreeSnapshot};
use hierarchy_application::common::ports::{
    SubtreeActionError, SubtreeAgentEnder, SubtreeAgentRemover, SubtreeWorktrees, WorktreeReport,
};
use hierarchy_domain::{Role, Team, TeamId, TeamName, TreeRow};
use kernel::AgentId;
use serde_json::json;
use wire::Request;

use super::*;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// These paths read and write only what their fakes hold.
fn never_called() -> ! {
    panic!("not used in these tests")
}

/// A store that always reads back the rows it was built with.
struct FixedRows {
    rows: Vec<TreeRow>,
    tree_writes: Mutex<()>,
}

impl FixedRows {
    fn holding(rows: Vec<TreeRow>) -> Self {
        Self {
            rows,
            tree_writes: Mutex::new(()),
        }
    }
}

impl HierarchyStore for FixedRows {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        Ok(TreeSnapshot {
            rows: self.rows.clone(),
            teams: Vec::new(),
        })
    }

    fn tree_exclusion(&self) -> TreeExclusion<'_> {
        TreeExclusion::new(&self.tree_writes)
    }

    fn team_name_taken(&self, _key: &str) -> Result<bool, HierarchyApplicationError> {
        never_called()
    }

    fn save_team_and_place_lead(
        &self,
        _team: &Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn delete_team_and_unplace(&self, _team_id: TeamId) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn place_agent(
        &self,
        _agent_id: AgentId,
        _role: Option<Role>,
        _parent_agent_id: Option<AgentId>,
        _team_id: Option<TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn team_name_of(
        &self,
        _team_id: TeamId,
    ) -> Result<Option<TeamName>, HierarchyApplicationError> {
        never_called()
    }
}

/// The agents the ender stopped, in order.
#[derive(Default)]
struct RecordingEnder {
    stopped: Mutex<Vec<AgentId>>,
}

impl SubtreeAgentEnder for RecordingEnder {
    fn stop(&self, agent_id: AgentId) -> Result<(), SubtreeActionError> {
        locked(&self.stopped).push(agent_id);
        Ok(())
    }
}

/// The agents the remover deleted, in order.
#[derive(Default)]
struct RecordingRemover {
    deleted: Mutex<Vec<AgentId>>,
}

impl SubtreeAgentRemover for RecordingRemover {
    fn delete(
        &self,
        agent_id: AgentId,
        worktrees: &SubtreeWorktrees,
    ) -> Result<WorktreeReport, SubtreeActionError> {
        locked(&self.deleted).push(agent_id);
        Ok(WorktreeReport {
            deleted_branches: if worktrees.remove {
                vec![agent_id.to_string()]
            } else {
                Vec::new()
            },
            ..WorktreeReport::default()
        })
    }
}

/// ceo > lead > member, the smallest tree with two levels below the root.
fn family() -> (Vec<TreeRow>, AgentId, AgentId, AgentId) {
    let ceo = AgentId::generate();
    let lead = AgentId::generate();
    let member = AgentId::generate();
    let row = |agent_id: AgentId, parent_agent_id: Option<AgentId>, role: Option<Role>| TreeRow {
        agent_id,
        name: agent_id.to_string(),
        role,
        parent_agent_id,
        team_id: None,
    };
    let rows = vec![
        row(ceo, None, Some(Role::Ceo)),
        row(lead, Some(ceo), Some(Role::Lead)),
        row(member, Some(lead), Some(Role::Member)),
    ];
    (rows, ceo, lead, member)
}

fn bridge(
    rows: &[TreeRow],
    ender: Arc<dyn SubtreeAgentEnder>,
    remover: Arc<dyn SubtreeAgentRemover>,
) -> SubtreeRequestsBridge {
    SubtreeRequestsBridge::new(SubtreeHandler::new(
        Arc::new(FixedRows::holding(rows.to_vec())),
        ender,
        remover,
    ))
}

fn request(kind: &str, fields: &serde_json::Value) -> Request {
    Request {
        id: 1,
        kind: kind.to_owned(),
        fields: fields.as_object().cloned().unwrap_or_default(),
    }
}

fn stopped_order(ender: &RecordingEnder) -> Vec<AgentId> {
    locked(&ender.stopped).clone()
}

fn deleted_order(remover: &RecordingRemover) -> Vec<AgentId> {
    locked(&remover.deleted).clone()
}

fn failure_of(result: SubtreeResult) -> (wire::ErrorCode, String) {
    match result {
        SubtreeResult::Failed { code, message } => (code, message),
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn a_stop_with_include_descendants_runs_deepest_first() {
    let (rows, ceo, lead, member) = family();
    let ender = Arc::new(RecordingEnder::default());
    let subtree = bridge(
        &rows,
        Arc::clone(&ender) as Arc<dyn SubtreeAgentEnder>,
        Arc::new(RecordingRemover::default()),
    );

    let result = subtree.stop_subtree(
        &request("agent.stop", &json!({ "include_descendants": true })),
        ceo,
    );

    assert_eq!(result, SubtreeResult::Stopped(vec![member, lead, ceo]));
    assert_eq!(stopped_order(&ender), [member, lead, ceo]);
}

#[test]
fn a_stop_without_include_descendants_stops_the_agent_alone() {
    let (rows, ceo, _lead, _member) = family();
    let ender = Arc::new(RecordingEnder::default());
    let subtree = bridge(
        &rows,
        Arc::clone(&ender) as Arc<dyn SubtreeAgentEnder>,
        Arc::new(RecordingRemover::default()),
    );

    let result = subtree.stop_subtree(&request("agent.stop", &json!({})), ceo);

    assert_eq!(result, SubtreeResult::Stopped(vec![ceo]));
    assert_eq!(stopped_order(&ender), [ceo]);
}

#[test]
fn a_stale_delete_count_becomes_the_wire_conflict() {
    let (rows, ceo, _lead, _member) = family();
    let subtree = bridge(
        &rows,
        Arc::new(RecordingEnder::default()),
        Arc::new(RecordingRemover::default()),
    );

    let result = subtree.delete_subtree(
        &request(
            "agent.delete",
            &json!({ "include_descendants": true, "descendant_count": 3 }),
        ),
        ceo,
        &DeleteWorktrees::Keep,
    );

    let (code, message) = failure_of(result);
    assert_eq!(code, wire::ErrorCode::Conflict);
    assert!(message.contains("has 2 descendants now"), "{message}");
}

#[test]
fn a_confirmed_delete_reports_the_deleted_agents() {
    let (rows, ceo, lead, member) = family();
    let remover = Arc::new(RecordingRemover::default());
    let subtree = bridge(
        &rows,
        Arc::new(RecordingEnder::default()),
        Arc::clone(&remover) as Arc<dyn SubtreeAgentRemover>,
    );

    let result = subtree.delete_subtree(
        &request(
            "agent.delete",
            &json!({ "include_descendants": true, "descendant_count": 2 }),
        ),
        ceo,
        &DeleteWorktrees::Remove(std::collections::HashMap::new()),
    );

    assert_eq!(
        result,
        SubtreeResult::Deleted {
            agents: vec![member, lead, ceo],
            worktrees: WorktreeOutcome {
                deleted_branches: vec![member.to_string(), lead.to_string(), ceo.to_string()],
                ..WorktreeOutcome::default()
            },
        }
    );
    assert_eq!(deleted_order(&remover), [member, lead, ceo]);
}
