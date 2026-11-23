// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Mutex;

use super::*;
use crate::common::interfaces::{TreeExclusion, TreeSnapshot};
use crate::common::ports::CreatorErrorKind;
use hierarchy_domain::{TeamId, TreeRow};

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The reads and writes these tests do not make.
fn never_called() -> ! {
    panic!("not used in these tests")
}

/// A store with a CEO row that records each saved team and fails the save
/// when `fail_save` is set.
struct TeamStore {
    rows: Vec<TreeRow>,
    name_taken: bool,
    fail_save: bool,
    saved: Mutex<Vec<Team>>,
    tree_writes: Mutex<()>,
}

impl TeamStore {
    fn with_ceo() -> Self {
        Self {
            rows: vec![TreeRow {
                agent_id: AgentId::generate(),
                name: "ceo".to_owned(),
                role: Some(Role::Ceo),
                parent_agent_id: None,
                team_id: None,
            }],
            name_taken: false,
            fail_save: false,
            saved: Mutex::new(Vec::new()),
            tree_writes: Mutex::new(()),
        }
    }

    fn saved(&self) -> Vec<Team> {
        locked(&self.saved).clone()
    }
}

impl HierarchyStore for TeamStore {
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
        Ok(self.name_taken)
    }

    fn save_team_and_place_lead_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
        team: &Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        if self.fail_save {
            return Err(HierarchyApplicationError::Storage(
                "the team write failed".to_owned(),
            ));
        }
        locked(&self.saved).push(team.clone());
        Ok(())
    }

    fn delete_team_and_unplace_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
        _team_id: TeamId,
    ) -> Result<(), HierarchyApplicationError> {
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

/// A creator that mints one id per create, records creates and removes,
/// and fails a call when the matching failure is set.
#[derive(Default)]
struct RecordingCreator {
    made: Mutex<Vec<AgentId>>,
    removed: Mutex<Vec<AgentId>>,
    fail_create: Option<CreatorError>,
    fail_remove: Option<CreatorError>,
}

impl AgentCreator for RecordingCreator {
    fn create(&self, _seed: AgentSeed) -> Result<AgentId, CreatorError> {
        if let Some(error) = self.fail_create.clone() {
            return Err(error);
        }
        let agent_id = AgentId::generate();
        locked(&self.made).push(agent_id);
        Ok(agent_id)
    }

    fn remove(&self, agent_id: AgentId) -> Result<(), CreatorError> {
        locked(&self.removed).push(agent_id);
        self.fail_remove.clone().map_or(Ok(()), Err)
    }
}

fn request() -> CreateTeamWithLeadRequest {
    CreateTeamWithLeadRequest {
        name: "api".to_owned(),
        project_id: None,
        lead: AgentSeed {
            temp: None,
            name: "api-lead".to_owned(),
            command: "claude".to_owned(),
            working_folder: None,
            project_id: None,
            additions: None,
            worktree: None,
        },
    }
}

fn handler(store: Arc<TeamStore>, creator: Arc<RecordingCreator>) -> CreateTeamHandler {
    CreateTeamHandler::new(store).with_creator(creator)
}

fn creator_error(message: &str) -> CreatorError {
    CreatorError {
        kind: CreatorErrorKind::Conflict,
        message: message.to_owned(),
    }
}

#[test]
fn a_new_lead_is_created_and_leads_the_saved_team() {
    let store = Arc::new(TeamStore::with_ceo());
    let creator = Arc::new(RecordingCreator::default());

    let team = handler(Arc::clone(&store), Arc::clone(&creator))
        .handle_with_new_lead(request())
        .expect("team");

    let made = locked(&creator.made).clone();
    assert_eq!(made, vec![team.lead_agent_id()]);
    assert_eq!(store.saved(), vec![team]);
    assert_eq!(*locked(&creator.removed), Vec::<AgentId>::new());
}

#[test]
fn a_taken_team_name_creates_no_lead() {
    let store = Arc::new(TeamStore {
        name_taken: true,
        ..TeamStore::with_ceo()
    });
    let creator = Arc::new(RecordingCreator::default());

    let error = handler(Arc::clone(&store), Arc::clone(&creator))
        .handle_with_new_lead(request())
        .expect_err("refused");

    assert!(
        matches!(error, HierarchyApplicationError::Conflict(_)),
        "{error}"
    );
    assert_eq!(*locked(&creator.made), Vec::<AgentId>::new());
    assert_eq!(store.saved(), Vec::<Team>::new());
}

#[test]
fn a_refused_lead_saves_no_team_and_keeps_the_agent_module_message() {
    let store = Arc::new(TeamStore::with_ceo());
    let creator = Arc::new(RecordingCreator {
        fail_create: Some(creator_error("agent name api-lead is already used")),
        ..RecordingCreator::default()
    });

    let error = handler(Arc::clone(&store), creator)
        .handle_with_new_lead(request())
        .expect_err("refused");

    assert_eq!(
        error,
        HierarchyApplicationError::ConflictInput("agent name api-lead is already used".to_owned())
    );
    assert_eq!(store.saved(), Vec::<Team>::new());
}

#[test]
fn a_failed_team_write_removes_the_new_lead() {
    let store = Arc::new(TeamStore {
        fail_save: true,
        ..TeamStore::with_ceo()
    });
    let creator = Arc::new(RecordingCreator::default());

    let error = handler(store, Arc::clone(&creator))
        .handle_with_new_lead(request())
        .expect_err("failed");

    assert!(
        matches!(error, HierarchyApplicationError::Storage(_)),
        "{error}"
    );
    assert_eq!(*locked(&creator.removed), *locked(&creator.made));
    assert_eq!(locked(&creator.made).len(), 1);
}

#[test]
fn a_failed_removal_names_the_leftover_lead() {
    let store = Arc::new(TeamStore {
        fail_save: true,
        ..TeamStore::with_ceo()
    });
    let creator = Arc::new(RecordingCreator {
        fail_remove: Some(creator_error("the agent store is locked")),
        ..RecordingCreator::default()
    });

    let error = handler(store, Arc::clone(&creator))
        .handle_with_new_lead(request())
        .expect_err("failed");

    let lead = locked(&creator.made)[0];
    let text = error.to_string();
    assert!(text.contains(&lead.to_string()), "{text}");
    assert!(text.contains("the agent store is locked"), "{text}");
}

#[test]
fn without_a_creator_a_new_lead_is_an_internal_failure() {
    let error = CreateTeamHandler::new(Arc::new(TeamStore::with_ceo()))
        .handle_with_new_lead(request())
        .expect_err("no creator");

    assert!(
        matches!(error, HierarchyApplicationError::Storage(_)),
        "{error}"
    );
}
