// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use super::*;
use crate::common::interfaces::TreeSnapshot;
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
            Err(HierarchyApplicationError::Storage)
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
/// creates and removes.
#[derive(Default)]
struct RecordingCreator {
    made: Mutex<Vec<AgentId>>,
    removed: Mutex<Vec<AgentId>>,
}

impl RecordingCreator {
    fn made(&self) -> Vec<AgentId> {
        locked(&self.made).clone()
    }

    fn removed(&self) -> Vec<AgentId> {
        locked(&self.removed).clone()
    }
}

impl AgentCreator for RecordingCreator {
    fn create(&self, _seed: AgentSeed) -> Result<AgentId, CreatorError> {
        let agent_id = AgentId::generate();
        locked(&self.made).push(agent_id);
        Ok(agent_id)
    }

    fn remove(&self, agent_id: AgentId) -> Result<(), CreatorError> {
        locked(&self.removed).push(agent_id);
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
) -> (
    CreateRoleAgentHandler,
    Arc<PlacingStore>,
    Arc<RecordingCreator>,
) {
    let store = Arc::new(PlacingStore {
        rows,
        fail_place,
        places: Mutex::new(Vec::new()),
    });
    let creator = Arc::new(RecordingCreator::default());
    let handler = CreateRoleAgentHandler::new(
        Arc::clone(&store) as Arc<dyn HierarchyStore>,
        Arc::clone(&creator) as Arc<dyn AgentCreator>,
    );
    (handler, store, creator)
}

#[test]
fn a_lead_with_a_team_is_created_and_then_placed() {
    let team = TeamId::from_u128(1);
    let (handler, store, creator) = handler_with(Vec::new(), false);

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
    let (handler, store, creator) = handler_with(family.rows, false);

    let agent_id = handler
        .handle(request(seed("Bo"), Role::Member, Some(family.lead), None))
        .expect("create and place");

    assert_eq!(creator.made(), [agent_id]);
    assert_eq!(store.placed(), [(agent_id, Some(family.team))]);
}

#[test]
fn a_second_ceo_is_refused_and_creates_nothing() {
    let ceo = AgentId::generate();
    let (handler, store, creator) = handler_with(vec![row(ceo, Some(Role::Ceo), None)], false);

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
    let (handler, store, creator) = handler_with(Vec::new(), true);

    let error = handler
        .handle(request(seed("Ana"), Role::Lead, None, Some(team)))
        .expect_err("the place write fails");

    assert_eq!(error, HierarchyApplicationError::Storage);
    assert_eq!(creator.made().len(), 1);
    assert_eq!(creator.removed(), creator.made());
    assert_eq!(store.placed().len(), 1);
}
