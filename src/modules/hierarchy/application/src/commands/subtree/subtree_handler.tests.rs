// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use super::*;
use hierarchy_domain::{Role, TreeRow};

use crate::common::interfaces::TreeSnapshot;
use crate::common::ports::SubtreeActionError;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The tree reads are the only store calls these tests make.
fn never_called() -> ! {
    panic!("not used in these tests")
}

/// A store that always reads back the rows it was built with.
struct FixedRows(Vec<TreeRow>);

impl HierarchyStore for FixedRows {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        Ok(TreeSnapshot {
            rows: self.0.clone(),
            teams: Vec::new(),
        })
    }

    fn team_name_taken(&self, _key: &str) -> Result<bool, HierarchyApplicationError> {
        never_called()
    }

    fn save_team_and_place_lead(
        &self,
        _team: &hierarchy_domain::Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn delete_team_and_unplace(
        &self,
        _team_id: hierarchy_domain::TeamId,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn place_agent(
        &self,
        _agent_id: AgentId,
        _role: Option<Role>,
        _parent_agent_id: Option<AgentId>,
        _team_id: Option<hierarchy_domain::TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn team_name_of(
        &self,
        _team_id: hierarchy_domain::TeamId,
    ) -> Result<Option<hierarchy_domain::TeamName>, HierarchyApplicationError> {
        never_called()
    }
}

#[derive(Default)]
struct Calls(Mutex<Vec<AgentId>>);

impl Calls {
    fn record(&self, agent_id: AgentId) {
        locked(&self.0).push(agent_id);
    }

    fn order(&self) -> Vec<AgentId> {
        locked(&self.0).clone()
    }
}

#[derive(Default)]
struct RecordingEnder {
    calls: Calls,
    failing: Option<AgentId>,
}

impl SubtreeAgentEnder for RecordingEnder {
    fn stop(&self, agent_id: AgentId) -> Result<(), SubtreeActionError> {
        self.calls.record(agent_id);
        if self.failing == Some(agent_id) {
            return Err(SubtreeActionError("the lead cannot be stopped".to_owned()));
        }
        Ok(())
    }
}

#[derive(Default)]
struct RecordingRemover {
    calls: Calls,
    failing: Option<AgentId>,
}

impl SubtreeAgentRemover for RecordingRemover {
    fn delete(&self, agent_id: AgentId) -> Result<(), SubtreeActionError> {
        self.calls.record(agent_id);
        if self.failing == Some(agent_id) {
            return Err(SubtreeActionError("the lead cannot be deleted".to_owned()));
        }
        Ok(())
    }
}

/// ceo > lead > member, the smallest tree with two levels below the root.
struct Family {
    ceo: AgentId,
    lead: AgentId,
    member: AgentId,
    rows: Vec<TreeRow>,
}

fn row(agent_id: AgentId, parent_agent_id: Option<AgentId>, role: Option<Role>) -> TreeRow {
    TreeRow {
        agent_id,
        role,
        parent_agent_id,
        team_id: None,
    }
}

fn family() -> Family {
    let ceo = AgentId::generate();
    let lead = AgentId::generate();
    let member = AgentId::generate();
    Family {
        ceo,
        lead,
        member,
        rows: vec![
            row(ceo, None, Some(Role::Ceo)),
            row(lead, Some(ceo), Some(Role::Lead)),
            row(member, Some(lead), Some(Role::Member)),
        ],
    }
}

fn request(
    agent_id: AgentId,
    include_descendants: bool,
    confirmed: Option<usize>,
) -> SubtreeRequest {
    SubtreeRequest {
        agent_id,
        include_descendants,
        confirmed_descendants: confirmed,
    }
}

fn handler(
    rows: &[TreeRow],
    ender: Arc<dyn SubtreeAgentEnder>,
    remover: Arc<dyn SubtreeAgentRemover>,
) -> SubtreeHandler {
    SubtreeHandler::new(Arc::new(FixedRows(rows.to_vec())), ender, remover)
}

#[test]
fn a_stop_runs_member_first_and_the_requested_agent_last() {
    let family = family();
    let ender = Arc::new(RecordingEnder::default());
    let handler = handler(
        &family.rows,
        Arc::clone(&ender) as Arc<dyn SubtreeAgentEnder>,
        Arc::new(RecordingRemover::default()),
    );

    let stopped = handler
        .stop_subtree(request(family.ceo, true, Some(2)))
        .expect("stop");

    assert_eq!(stopped, [family.member, family.lead, family.ceo]);
    assert_eq!(
        ender.calls.order(),
        [family.member, family.lead, family.ceo]
    );
}

#[test]
fn a_delete_runs_member_first_and_the_requested_agent_last() {
    let family = family();
    let remover = Arc::new(RecordingRemover::default());
    let handler = handler(
        &family.rows,
        Arc::new(RecordingEnder::default()),
        Arc::clone(&remover) as Arc<dyn SubtreeAgentRemover>,
    );

    let deleted = handler
        .delete_subtree(request(family.ceo, true, Some(2)))
        .expect("delete");

    assert_eq!(deleted, [family.member, family.lead, family.ceo]);
    assert_eq!(
        remover.calls.order(),
        [family.member, family.lead, family.ceo]
    );
}

#[test]
fn a_failing_delete_deletes_no_agent_above_the_failed_one() {
    let family = family();
    let remover = Arc::new(RecordingRemover {
        failing: Some(family.lead),
        ..Default::default()
    });
    let handler = handler(
        &family.rows,
        Arc::new(RecordingEnder::default()),
        Arc::clone(&remover) as Arc<dyn SubtreeAgentRemover>,
    );

    let error = handler
        .delete_subtree(request(family.ceo, true, Some(2)))
        .expect_err("the lead cannot be deleted");

    assert!(
        matches!(&error, HierarchyApplicationError::Conflict(text) if text.contains("the lead cannot be deleted"))
    );
    // The member went first; the root was never attempted, so it still exists.
    assert_eq!(remover.calls.order(), [family.member, family.lead]);
}

#[test]
fn an_agent_without_descendants_deletes_alone_without_a_confirmation() {
    let agent_id = AgentId::generate();
    let remover = Arc::new(RecordingRemover::default());
    let handler = handler(
        &[row(agent_id, None, Some(Role::Ceo))],
        Arc::new(RecordingEnder::default()),
        Arc::clone(&remover) as Arc<dyn SubtreeAgentRemover>,
    );

    let deleted = handler
        .delete_subtree(request(agent_id, false, None))
        .expect("delete");

    assert_eq!(deleted, [agent_id]);
    assert_eq!(remover.calls.order(), [agent_id]);
}
