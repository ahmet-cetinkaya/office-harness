// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use super::*;
use hierarchy_domain::{Role, TreeRow};

use crate::common::interfaces::{TreeExclusion, TreeSnapshot};
use crate::common::ports::{SubtreeActionError, SubtreeWorktrees, WorktreeReport};

/// The name a test row carries: the id text, since these rules do not read
/// names (only the CEO refusal does, and its test names its row itself).
fn name_of(agent_id: AgentId) -> String {
    agent_id.to_string()
}

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

    fn save_team_and_place_lead_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
        _team: &hierarchy_domain::Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn delete_team_and_unplace_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
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
    /// The agent whose worktree check refuses.
    stale: Option<AgentId>,
    /// The agent whose worktree changes while it stops.
    changes_at_stop: Option<AgentId>,
}

impl SubtreeAgentRemover for RecordingRemover {
    fn check(
        &self,
        agent_id: AgentId,
        _worktrees: &SubtreeWorktrees,
    ) -> Result<(), SubtreeActionError> {
        if self.stale == Some(agent_id) {
            return Err(SubtreeActionError(
                "the worktree of the member changed; confirm again".to_owned(),
            ));
        }
        Ok(())
    }

    fn prepare(
        &self,
        agent_id: AgentId,
        _worktrees: &SubtreeWorktrees,
    ) -> Result<(), SubtreeActionError> {
        if self.changes_at_stop == Some(agent_id) {
            return Err(SubtreeActionError(
                "the worktree of the member changed while it stopped".to_owned(),
            ));
        }
        Ok(())
    }

    fn delete(
        &self,
        agent_id: AgentId,
        _worktrees: &SubtreeWorktrees,
    ) -> Result<WorktreeReport, SubtreeActionError> {
        self.calls.record(agent_id);
        if self.failing == Some(agent_id) {
            return Err(SubtreeActionError("the lead cannot be deleted".to_owned()));
        }
        Ok(WorktreeReport {
            kept_branches: vec![format!("office-harness/{}", name_of(agent_id))],
            ..WorktreeReport::default()
        })
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
        name: name_of(agent_id),
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
        worktrees: SubtreeWorktrees::default(),
    }
}

fn handler(
    rows: &[TreeRow],
    ender: Arc<dyn SubtreeAgentEnder>,
    remover: Arc<dyn SubtreeAgentRemover>,
) -> SubtreeHandler {
    SubtreeHandler::new(Arc::new(FixedRows::holding(rows.to_vec())), ender, remover)
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
        .stop_subtree(&request(family.ceo, true, Some(2)))
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
        .delete_subtree(&request(family.ceo, true, Some(2)))
        .expect("delete");

    assert_eq!(deleted.deleted, [family.member, family.lead, family.ceo]);
    // One report covers every deleted agent's worktree (0010 FR-005).
    assert_eq!(deleted.worktrees.kept_branches.len(), 3);
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
        .delete_subtree(&request(family.ceo, true, Some(2)))
        .expect_err("the lead cannot be deleted");

    let HierarchyApplicationError::ConflictInput(text) = &error else {
        panic!("{error:?}");
    };
    assert!(text.contains("the lead cannot be deleted"), "{text}");
    // The member's kept branch is reported once, in the refusal.
    let member_branch = format!("office-harness/{}", name_of(family.member));
    assert!(text.contains(&member_branch), "{text}");
    assert!(!text.contains(&name_of(family.lead)), "{text}");
    // The member went first; the root was never attempted, so it still exists.
    assert_eq!(remover.calls.order(), [family.member, family.lead]);
}

#[test]
fn a_stale_worktree_on_one_member_refuses_before_any_delete() {
    let family = family();
    let remover = Arc::new(RecordingRemover {
        stale: Some(family.lead),
        ..Default::default()
    });
    let handler = handler(
        &family.rows,
        Arc::new(RecordingEnder::default()),
        Arc::clone(&remover) as Arc<dyn SubtreeAgentRemover>,
    );

    let error = handler
        .delete_subtree(&request(family.ceo, true, Some(2)))
        .expect_err("stale");

    assert!(
        matches!(&error, HierarchyApplicationError::ConflictInput(text) if text.contains("confirm again"))
    );
    assert_eq!(remover.calls.order(), [] as [kernel::AgentId; 0]);
}

#[test]
fn a_member_that_changes_its_worktree_at_its_stop_refuses_before_any_delete() {
    let family = family();
    let remover = Arc::new(RecordingRemover {
        changes_at_stop: Some(family.lead),
        ..Default::default()
    });
    let handler = handler(
        &family.rows,
        Arc::new(RecordingEnder::default()),
        Arc::clone(&remover) as Arc<dyn SubtreeAgentRemover>,
    );

    let error = handler
        .delete_subtree(&request(family.ceo, true, Some(2)))
        .expect_err("changed at stop");

    assert!(matches!(
        &error,
        HierarchyApplicationError::ConflictInput(_)
    ));
    assert_eq!(remover.calls.order().len(), 0);
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
        .delete_subtree(&request(agent_id, false, None))
        .expect("delete");

    assert_eq!(deleted.deleted, [agent_id]);
    assert_eq!(remover.calls.order(), [agent_id]);
}

/// A ceo > lead tree whose store places a second child under the CEO during
/// the delete, exactly in the window between the count read and the delete
/// loop that a concurrent team creation could win. It also records whether
/// any delete ran while another writer held the exclusion.
struct GrowingTree {
    rows: Mutex<Vec<TreeRow>>,
    late_child: AgentId,
    tree_writes: Mutex<()>,
    /// Whether a delete ran without holding the exclusion, taken while a
    /// placement was mid-write.
    exclusion_leaked: Mutex<bool>,
}

impl GrowingTree {
    fn new(rows: Vec<TreeRow>, late_child: AgentId) -> Self {
        Self {
            rows: Mutex::new(rows),
            late_child,
            tree_writes: Mutex::new(()),
            exclusion_leaked: Mutex::new(false),
        }
    }

    /// Proves the caller of the delete holds the exclusion: the writer's
    /// lock cannot be taken, so a concurrent placement is not mid-write
    /// while the count is read or a delete runs.
    fn assert_exclusive(&self) {
        let leaked = self.tree_writes.try_lock().is_ok();
        *locked(&self.exclusion_leaked) |= leaked;
    }
}

impl HierarchyStore for GrowingTree {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        self.assert_exclusive();
        Ok(TreeSnapshot {
            rows: locked(&self.rows).clone(),
            teams: Vec::new(),
        })
    }

    fn tree_exclusion(&self) -> TreeExclusion<'_> {
        TreeExclusion::new(&self.tree_writes)
    }

    fn team_name_taken(&self, _key: &str) -> Result<bool, HierarchyApplicationError> {
        never_called()
    }

    fn save_team_and_place_lead_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
        _team: &hierarchy_domain::Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn delete_team_and_unplace_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
        _team_id: hierarchy_domain::TeamId,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn place_agent(
        &self,
        agent_id: AgentId,
        role: Option<Role>,
        parent_agent_id: Option<AgentId>,
        _team_id: Option<hierarchy_domain::TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        // A second child joins under the CEO while the first delete is in
        // flight, the placement a concurrent team creation would win.
        locked(&self.rows).push(row(agent_id, parent_agent_id, role));
        Ok(())
    }

    fn team_name_of(
        &self,
        _team_id: hierarchy_domain::TeamId,
    ) -> Result<Option<hierarchy_domain::TeamName>, HierarchyApplicationError> {
        never_called()
    }
}

/// The remover that deletes the lead and, at that exact moment, places the
/// second child the way a racing placement would.
struct GrowingRemover {
    calls: Calls,
    ceo: AgentId,
    store: Arc<GrowingTree>,
}

impl SubtreeAgentRemover for GrowingRemover {
    fn check(
        &self,
        _agent_id: AgentId,
        _worktrees: &SubtreeWorktrees,
    ) -> Result<(), SubtreeActionError> {
        Ok(())
    }

    fn prepare(
        &self,
        _agent_id: AgentId,
        _worktrees: &SubtreeWorktrees,
    ) -> Result<(), SubtreeActionError> {
        Ok(())
    }

    fn delete(
        &self,
        agent_id: AgentId,
        _worktrees: &SubtreeWorktrees,
    ) -> Result<WorktreeReport, SubtreeActionError> {
        self.store.assert_exclusive();
        if self.calls.order().is_empty() {
            self.store
                .place_agent(
                    self.store.late_child,
                    Some(Role::Member),
                    Some(self.ceo),
                    None,
                )
                .expect("place the second child");
        }
        self.calls.record(agent_id);
        Ok(WorktreeReport::default())
    }
}

#[test]
fn a_delete_confirmed_on_one_tree_deletes_no_child_placed_after_its_count() {
    let ceo = AgentId::generate();
    let lead = AgentId::generate();
    let late_child = AgentId::generate();
    let rows = vec![
        row(ceo, None, Some(Role::Ceo)),
        row(lead, Some(ceo), Some(Role::Lead)),
    ];
    let store = Arc::new(GrowingTree::new(rows, late_child));
    let remover = Arc::new(GrowingRemover {
        calls: Calls::default(),
        ceo,
        store: Arc::clone(&store),
    });
    let handler = SubtreeHandler::new(
        Arc::clone(&store) as Arc<dyn HierarchyStore>,
        Arc::new(RecordingEnder::default()),
        Arc::clone(&remover) as Arc<dyn SubtreeAgentRemover>,
    );

    // The confirmed count of 1 matches the tree as the delete first read
    // it. The old code read that count in one snapshot and deleted in
    // another, so the child placed in between was missed and left orphaned;
    // holding the exclusion across both keeps the confirmed tree the one
    // that is deleted.
    let deleted = handler
        .delete_subtree(&request(ceo, true, Some(1)))
        .expect("delete");

    assert_eq!(deleted.deleted, [lead, ceo]);
    assert_eq!(remover.calls.order(), [lead, ceo]);
    assert!(
        !*locked(&store.exclusion_leaked),
        "the delete read or ran a step while another tree writer held the exclusion"
    );
    let final_tree = locked(&store.rows);
    // The exclusion released only after the deletes, so the child that
    // joined mid-delete is still there, to be confirmed on a later request
    // instead of deleted unseen.
    assert!(
        final_tree
            .iter()
            .any(|tree_row| tree_row.agent_id == late_child)
    );
}
