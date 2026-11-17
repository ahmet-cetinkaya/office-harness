// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use super::*;
use crate::common::interfaces::{TreeExclusion, TreeSnapshot};
use crate::common::ports::CreatorErrorKind;
use hierarchy_domain::{HierarchyError, TreeRow};

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The reads and writes these tests do not make.
fn never_called() -> ! {
    panic!("not used in these tests")
}

/// A store that reads back the rows it was built with, records every place
/// call, and fails them when `fail_place` is set.
struct PlacingStore {
    rows: Vec<TreeRow>,
    fail_place: bool,
    places: Mutex<Vec<(AgentId, Option<TeamId>)>>,
    /// Never contended here; the exclusion exists so the trait stays whole.
    tree_writes: Mutex<()>,
}

impl PlacingStore {
    fn placed(&self) -> Vec<(AgentId, Option<TeamId>)> {
        locked(&self.places).clone()
    }
}

impl HierarchyStore for PlacingStore {
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
        _team: &hierarchy_domain::Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn delete_team_and_unplace(&self, _team_id: TeamId) -> Result<(), HierarchyApplicationError> {
        never_called()
    }

    fn place_agent(
        &self,
        agent_id: AgentId,
        _role: Option<Role>,
        _parent_agent_id: Option<AgentId>,
        team_id: Option<TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        locked(&self.places).push((agent_id, team_id));
        if self.fail_place {
            Err(HierarchyApplicationError::Storage(
                "the tree write failed".to_owned(),
            ))
        } else {
            Ok(())
        }
    }

    fn team_name_of(
        &self,
        _team_id: TeamId,
    ) -> Result<Option<hierarchy_domain::TeamName>, HierarchyApplicationError> {
        never_called()
    }
}

/// A creator that mints a fresh id per create and records the order of
/// creates and removes. It fails a call when the matching failure is set.
#[derive(Default)]
struct RecordingCreator {
    made: Mutex<Vec<AgentId>>,
    removed: Mutex<Vec<AgentId>>,
    fail_create: Mutex<Option<CreatorError>>,
    fail_remove: Mutex<Option<CreatorError>>,
}

impl RecordingCreator {
    fn made(&self) -> Vec<AgentId> {
        locked(&self.made).clone()
    }

    fn removed(&self) -> Vec<AgentId> {
        locked(&self.removed).clone()
    }

    fn failing_create(error: CreatorError) -> Self {
        Self {
            fail_create: Mutex::new(Some(error)),
            ..Self::default()
        }
    }

    fn failing_remove(error: CreatorError) -> Self {
        Self {
            fail_remove: Mutex::new(Some(error)),
            ..Self::default()
        }
    }
}

impl AgentCreator for RecordingCreator {
    fn create(&self, _seed: AgentSeed) -> Result<AgentId, CreatorError> {
        if let Some(error) = locked(&self.fail_create).clone() {
            return Err(error);
        }
        let agent_id = AgentId::generate();
        locked(&self.made).push(agent_id);
        Ok(agent_id)
    }

    fn remove(&self, agent_id: AgentId) -> Result<(), CreatorError> {
        locked(&self.removed).push(agent_id);
        if let Some(error) = locked(&self.fail_remove).clone() {
            return Err(error);
        }
        Ok(())
    }
}

/// A lead with a team, for a member that joins it.
struct TeamWithLead {
    lead: AgentId,
    team: TeamId,
    rows: Vec<TreeRow>,
}

fn row(agent_id: AgentId, role: Option<Role>, team_id: Option<TeamId>) -> TreeRow {
    TreeRow {
        agent_id,
        role,
        parent_agent_id: None,
        team_id,
    }
}

fn lead_with_team() -> TeamWithLead {
    let lead = AgentId::generate();
    let team = TeamId::from_u128(2);
    TeamWithLead {
        lead,
        team,
        rows: vec![row(lead, Some(Role::Lead), Some(team))],
    }
}

fn seed(name: &str) -> AgentSeed {
    AgentSeed {
        name: name.to_owned(),
        command: "claude".to_owned(),
        working_folder: None,
        project_id: None,
        additions: None,
    }
}

fn request(
    seed: AgentSeed,
    role: Role,
    parent_agent_id: Option<AgentId>,
    team_id: Option<TeamId>,
) -> CreateRoleAgentRequest {
    CreateRoleAgentRequest {
        seed,
        role,
        parent_agent_id,
        team_id,
    }
}

/// Builds the handler over its store and hands both back, so a test can
/// read the places the handler wrote.
fn handler_with(
    rows: Vec<TreeRow>,
    fail_place: bool,
    creator: RecordingCreator,
) -> (
    CreateRoleAgentHandler,
    Arc<PlacingStore>,
    Arc<RecordingCreator>,
) {
    let store = Arc::new(PlacingStore {
        rows,
        fail_place,
        places: Mutex::new(Vec::new()),
        tree_writes: Mutex::new(()),
    });
    let creator = Arc::new(creator);
    let handler = CreateRoleAgentHandler::new(
        Arc::clone(&store) as Arc<dyn HierarchyStore>,
        Arc::clone(&creator) as Arc<dyn AgentCreator>,
    );
    (handler, store, creator)
}

#[test]
fn a_lead_with_a_team_is_created_and_then_placed() {
    let team = TeamId::from_u128(1);
    let (handler, store, creator) = handler_with(Vec::new(), false, RecordingCreator::default());

    let agent_id = handler
        .handle(request(seed("Ana"), Role::Lead, None, Some(team)))
        .expect("create and place");

    assert_eq!(creator.made(), [agent_id]);
    assert_eq!(creator.removed(), Vec::<AgentId>::new());
    assert_eq!(store.placed(), [(agent_id, Some(team))]);
}

#[test]
fn a_member_takes_its_leads_team() {
    let family = lead_with_team();
    let (handler, store, creator) = handler_with(family.rows, false, RecordingCreator::default());

    let agent_id = handler
        .handle(request(seed("Bo"), Role::Member, Some(family.lead), None))
        .expect("create and place");

    assert_eq!(creator.made(), [agent_id]);
    assert_eq!(store.placed(), [(agent_id, Some(family.team))]);
}

#[test]
fn a_second_ceo_is_refused_and_creates_nothing() {
    let ceo = AgentId::generate();
    let (handler, store, creator) = handler_with(
        vec![row(ceo, Some(Role::Ceo), None)],
        false,
        RecordingCreator::default(),
    );

    let error = handler
        .handle(request(seed("Sid"), Role::Ceo, None, None))
        .expect_err("the CEO seat is taken");

    assert!(matches!(
        error,
        HierarchyApplicationError::Invalid(HierarchyError::SecondCeo { .. })
    ));
    assert_eq!(creator.made(), Vec::<AgentId>::new());
    assert_eq!(creator.removed(), Vec::<AgentId>::new());
    assert_eq!(store.placed(), Vec::<(AgentId, Option<TeamId>)>::new());
}

#[test]
fn a_failed_placement_removes_the_created_agent() {
    let team = TeamId::from_u128(3);
    let (handler, store, creator) = handler_with(Vec::new(), true, RecordingCreator::default());

    let error = handler
        .handle(request(seed("Ana"), Role::Lead, None, Some(team)))
        .expect_err("the place write fails");

    assert_eq!(
        error,
        HierarchyApplicationError::Storage("the tree write failed".to_owned())
    );
    assert_eq!(creator.made().len(), 1);
    assert_eq!(creator.removed(), creator.made());
    assert_eq!(store.placed().len(), 1);
}

#[test]
fn a_failed_removal_after_a_failed_placement_names_both_failures() {
    let team = TeamId::from_u128(3);
    let creator = RecordingCreator::failing_remove(CreatorError {
        kind: CreatorErrorKind::Internal,
        message: "the agent store is closed".to_owned(),
    });
    let (handler, _store, creator) = handler_with(Vec::new(), true, creator);

    let error = handler
        .handle(request(seed("Ana"), Role::Lead, None, Some(team)))
        .expect_err("the place write fails and the removal with it");

    let leftover = creator.made()[0];
    assert_eq!(
        error,
        HierarchyApplicationError::Storage(format!(
            "the tree write failed; the unplaced agent {leftover} \
             could not be removed either: the agent store is closed"
        ))
    );
    assert_eq!(creator.removed(), creator.made());
}

/// Creates with each kind of refusal and expects the application error of
/// the same kind, so a seeded create answers with the code `agent.create`
/// answers with and keeps the agent module's message.
#[test]
fn a_refused_create_maps_to_the_matching_error_kind() {
    let refusals = [
        (CreatorErrorKind::Invalid, "agent name must not be empty"),
        (CreatorErrorKind::NotFound, "the project is gone"),
        (CreatorErrorKind::Conflict, "an agent named Ana exists"),
        (CreatorErrorKind::Internal, "the agent store is closed"),
    ];
    for (kind, message) in refusals {
        let message = message.to_owned();
        let expected = match kind {
            CreatorErrorKind::Invalid => HierarchyApplicationError::InvalidInput(message.clone()),
            CreatorErrorKind::NotFound => HierarchyApplicationError::NotFound(message.clone()),
            CreatorErrorKind::Conflict => HierarchyApplicationError::Conflict(message.clone()),
            CreatorErrorKind::Internal => HierarchyApplicationError::Storage(message.clone()),
        };
        let creator = RecordingCreator::failing_create(CreatorError { kind, message });
        let (handler, store, recorded) = handler_with(Vec::new(), false, creator);

        let error = handler
            .handle(request(
                seed("Ana"),
                Role::Lead,
                None,
                Some(TeamId::from_u128(1)),
            ))
            .expect_err("the create is refused");

        assert_eq!(error, expected);
        assert_eq!(recorded.made(), Vec::<AgentId>::new());
        assert_eq!(recorded.removed(), Vec::<AgentId>::new());
        assert_eq!(store.placed(), Vec::<(AgentId, Option<TeamId>)>::new());
    }
}
