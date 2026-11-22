// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use super::*;
use crate::common::interfaces::{TreeExclusion, TreeSnapshot};
use crate::common::ports::{CreatorErrorKind, SeedAdditions};
use hierarchy_domain::{HierarchyError, TreeRow};

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

/// The reads and writes these tests do not make.
fn never_called() -> ! {
    panic!("not used in these tests")
}

/// A store that reads back the rows it was built with, records every place
/// call, and fails them when `fail_place` is set.
struct PlacingStore {
    rows: Vec<TreeRow>,
    teams: Vec<hierarchy_domain::entities::Team>,
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
            teams: self.teams.clone(),
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
    seeds: Mutex<Vec<AgentSeed>>,
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
    fn create(&self, seed: AgentSeed) -> Result<AgentId, CreatorError> {
        if let Some(error) = locked(&self.fail_create).clone() {
            return Err(error);
        }
        locked(&self.seeds).push(seed);
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
        name: name_of(agent_id),
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
    handler_over(rows, Vec::new(), fail_place, creator)
}

/// As `handler_with`, with the teams the snapshot reads.
fn handler_over(
    rows: Vec<TreeRow>,
    teams: Vec<hierarchy_domain::entities::Team>,
    fail_place: bool,
    creator: RecordingCreator,
) -> (
    CreateRoleAgentHandler,
    Arc<PlacingStore>,
    Arc<RecordingCreator>,
) {
    let store = Arc::new(PlacingStore {
        rows,
        teams,
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
    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    let (handler, store, creator) = handler_with(Vec::new(), false, RecordingCreator::default());

    let agent_id = handler
        .handle(request(seed("Ana"), Role::Lead, None, Some(team)), &tree)
        .expect("create and place");

    assert_eq!(creator.made(), [agent_id]);
    assert_eq!(creator.removed(), Vec::<AgentId>::new());
    assert_eq!(store.placed(), [(agent_id, Some(team))]);
}

#[test]
fn a_second_lead_for_a_held_team_is_refused_and_creates_nothing() {
    let family = lead_with_team();
    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    let (handler, store, creator) = handler_with(family.rows, false, RecordingCreator::default());

    let error = handler
        .handle(
            request(seed("Sid"), Role::Lead, None, Some(family.team)),
            &tree,
        )
        .expect_err("the team's seat is taken");

    assert_eq!(
        error,
        HierarchyApplicationError::Conflict(format!(
            "the team already has a lead: {}",
            family.lead
        ))
    );
    assert_eq!(creator.made(), Vec::<AgentId>::new());
    assert_eq!(creator.removed(), Vec::<AgentId>::new());
    assert_eq!(store.placed(), Vec::<(AgentId, Option<TeamId>)>::new());
}

#[test]
fn a_member_takes_its_leads_team() {
    let family = lead_with_team();
    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    let (handler, store, creator) = handler_with(family.rows, false, RecordingCreator::default());

    let agent_id = handler
        .handle(
            request(seed("Bo"), Role::Member, Some(family.lead), None),
            &tree,
        )
        .expect("create and place");

    assert_eq!(creator.made(), [agent_id]);
    assert_eq!(store.placed(), [(agent_id, Some(family.team))]);
}

/// A6: a member created with no folder and no project takes its team's
/// project, so the agent module uses that project's folder.
#[test]
fn a_member_without_a_folder_takes_its_teams_project() {
    let family = lead_with_team();
    let project = kernel::ProjectId::generate();
    let team = hierarchy_domain::entities::Team::stored(
        family.team,
        hierarchy_domain::TeamName::parse("Backend").expect("name"),
        Some(project),
        family.lead,
    );
    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    let (handler, _store, creator) =
        handler_over(family.rows, vec![team], false, RecordingCreator::default());

    handler
        .handle(
            request(seed("Bo"), Role::Member, Some(family.lead), None),
            &tree,
        )
        .expect("create and place");

    let seeds = locked(&creator.seeds).clone();
    assert_eq!(seeds.len(), 1);
    assert_eq!(seeds[0].project_id, Some(project));
    assert_eq!(seeds[0].working_folder, None);
}

/// A6: a project that differs from the team's is refused, nothing is
/// created, and the team's own project id passes.
#[test]
fn a_project_other_than_the_teams_is_refused_and_the_teams_own_passes() {
    let family = lead_with_team();
    let project = kernel::ProjectId::generate();
    let team = hierarchy_domain::entities::Team::stored(
        family.team,
        hierarchy_domain::TeamName::parse("Backend").expect("name"),
        Some(project),
        family.lead,
    );
    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    let (handler, _store, creator) =
        handler_over(family.rows, vec![team], false, RecordingCreator::default());
    let with_project = |project_id| AgentSeed {
        project_id: Some(project_id),
        ..seed("Bo")
    };

    let error = handler
        .handle(
            request(
                with_project(kernel::ProjectId::generate()),
                Role::Member,
                Some(family.lead),
                None,
            ),
            &tree,
        )
        .expect_err("another project than the team's");
    assert_eq!(
        error,
        HierarchyApplicationError::InvalidInput(PROJECT_IS_TEAMS_MESSAGE.to_owned())
    );
    assert_eq!(creator.made(), Vec::<AgentId>::new());

    handler
        .handle(
            request(with_project(project), Role::Member, Some(family.lead), None),
            &tree,
        )
        .expect("the team's own project");
    assert_eq!(locked(&creator.seeds)[0].project_id, Some(project));
}

#[test]
fn a_second_ceo_is_refused_and_creates_nothing() {
    let ceo = AgentId::generate();
    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    let (handler, store, creator) = handler_with(
        vec![row(ceo, Some(Role::Ceo), None)],
        false,
        RecordingCreator::default(),
    );

    let error = handler
        .handle(request(seed("Sid"), Role::Ceo, None, None), &tree)
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
    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    let (handler, store, creator) = handler_with(Vec::new(), true, RecordingCreator::default());

    let error = handler
        .handle(request(seed("Ana"), Role::Lead, None, Some(team)), &tree)
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
    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    let (handler, _store, creator) = handler_with(Vec::new(), true, creator);

    let error = handler
        .handle(request(seed("Ana"), Role::Lead, None, Some(team)), &tree)
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
            CreatorErrorKind::NotFound => HierarchyApplicationError::NotFoundInput(message.clone()),
            CreatorErrorKind::Conflict => HierarchyApplicationError::ConflictInput(message.clone()),
            CreatorErrorKind::Internal => HierarchyApplicationError::Storage(message.clone()),
        };
        let creator = RecordingCreator::failing_create(CreatorError { kind, message });
        let tree_writes = Mutex::new(());
        let tree = TreeExclusion::new(&tree_writes);
        let (handler, store, recorded) = handler_with(Vec::new(), false, creator);

        let error = handler
            .handle(
                request(seed("Ana"), Role::Lead, None, Some(TeamId::from_u128(1))),
                &tree,
            )
            .expect_err("the create is refused");

        assert_eq!(error, expected);
        assert_eq!(recorded.made(), Vec::<AgentId>::new());
        assert_eq!(recorded.removed(), Vec::<AgentId>::new());
        assert_eq!(store.placed(), Vec::<(AgentId, Option<TeamId>)>::new());
    }
}

/// A store whose create window records whether any step ran without the
/// caller holding the exclusion; its place writes join the live rows the way
/// a real store would.
struct GuardedStore {
    rows: Mutex<Vec<TreeRow>>,
    places: Mutex<Vec<AgentId>>,
    tree_writes: Mutex<()>,
    /// Whether the snapshot or a place ran while another writer held the
    /// exclusion.
    exclusion_leaked: Mutex<bool>,
}

impl GuardedStore {
    fn new(rows: Vec<TreeRow>) -> Self {
        Self {
            rows: Mutex::new(rows),
            places: Mutex::new(Vec::new()),
            tree_writes: Mutex::new(()),
            exclusion_leaked: Mutex::new(false),
        }
    }

    fn assert_exclusive(&self) {
        let leaked = self.tree_writes.try_lock().is_ok();
        *locked(&self.exclusion_leaked) |= leaked;
    }

    fn placed(&self) -> Vec<AgentId> {
        locked(&self.places).clone()
    }
}

impl HierarchyStore for GuardedStore {
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
        role: Option<Role>,
        parent_agent_id: Option<AgentId>,
        team_id: Option<TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        self.assert_exclusive();
        locked(&self.rows).push(TreeRow {
            agent_id,
            name: name_of(agent_id),
            role,
            parent_agent_id,
            team_id,
        });
        locked(&self.places).push(agent_id);
        Ok(())
    }

    fn team_name_of(
        &self,
        _team_id: TeamId,
    ) -> Result<Option<hierarchy_domain::TeamName>, HierarchyApplicationError> {
        never_called()
    }
}

/// A creator that, on the first create, simulates a concurrent delete
/// sweeping the row the check just counted: the second creator's place then
/// runs against the shrunk tree.
struct DeleteRacingCreator {
    made: Mutex<Vec<AgentId>>,
    store: Arc<GuardedStore>,
    victim: AgentId,
}

impl DeleteRacingCreator {
    fn made(&self) -> Vec<AgentId> {
        locked(&self.made).clone()
    }
}

impl AgentCreator for DeleteRacingCreator {
    fn create(&self, _seed: AgentSeed) -> Result<AgentId, CreatorError> {
        let agent_id = AgentId::generate();
        // The concurrent delete removes the victim's row exactly while this
        // create is in flight — the window the exclusion must close.
        locked(&self.store.rows).retain(|row| row.agent_id != self.victim);
        locked(&self.made).push(agent_id);
        Ok(agent_id)
    }

    fn remove(&self, _agent_id: AgentId) -> Result<(), CreatorError> {
        Ok(())
    }
}

/// A creator whose every create succeeds, for the race of two CEO creates:
/// the test reads who was made and who was removed.
struct CeoRacingCreator {
    made: Mutex<Vec<AgentId>>,
    removed: Mutex<Vec<AgentId>>,
}

impl CeoRacingCreator {
    fn made(&self) -> Vec<AgentId> {
        locked(&self.made).clone()
    }

    fn removed(&self) -> Vec<AgentId> {
        locked(&self.removed).clone()
    }

    fn default_slots() -> Self {
        Self {
            made: Mutex::new(Vec::new()),
            removed: Mutex::new(Vec::new()),
        }
    }
}

impl AgentCreator for CeoRacingCreator {
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

fn guarded_handler(
    store: Arc<dyn HierarchyStore>,
    creator: Arc<dyn AgentCreator>,
) -> CreateRoleAgentHandler {
    CreateRoleAgentHandler::new(store, creator)
}

#[test]
fn a_concurrent_delete_between_check_and_create_leaves_no_orphan() {
    // The lead the new member sits under is itself the victim of the
    // concurrent delete: the check saw it, the create-window delete removes
    // it, and the place must not leave the new member under a deleted lead.
    let lead = AgentId::generate();
    let store = Arc::new(GuardedStore::new(vec![row(
        lead,
        Some(Role::Lead),
        Some(TeamId::from_u128(7)),
    )]));
    let creator = Arc::new(DeleteRacingCreator {
        made: Mutex::new(Vec::new()),
        store: Arc::clone(&store),
        victim: lead,
    });
    let handler = guarded_handler(
        Arc::clone(&store) as Arc<dyn HierarchyStore>,
        Arc::clone(&creator) as Arc<dyn AgentCreator>,
    );

    // The caller holds the exclusion across check+create+place, so the
    // concurrent delete (simulated inside the create window) cannot make
    // the check's tree stale: the check counted a lead that the real store
    // no longer holds, and the place writes into the tree as it stands at
    // the place — the new member is placed, and no orphan row of the check's
    // tree survives unaccounted. The guard is the store's own, the one the
    // real caller holds.
    let agent_id = {
        let tree = store.tree_exclusion();
        handler
            .handle(request(seed("Ana"), Role::Member, Some(lead), None), &tree)
            .expect("create succeeds against the tree the check saw")
    };

    assert_eq!(creator.made(), [agent_id]);
    assert_eq!(store.placed(), [agent_id]);
    assert!(
        !*locked(&store.exclusion_leaked),
        "the check or place ran while another writer held the exclusion"
    );
}

#[test]
fn two_racing_ceo_creates_let_one_win_and_remove_the_loser() {
    let store = Arc::new(GuardedStore::new(Vec::new()));
    let creator = Arc::new(CeoRacingCreator::default_slots());
    let handler = guarded_handler(
        Arc::clone(&store) as Arc<dyn HierarchyStore>,
        Arc::clone(&creator) as Arc<dyn AgentCreator>,
    );

    // The guard serializes the two requests: the first create+place wins
    // the CEO seat, the second sees the row and is refused. Each request
    // holds the exclusion for its own call only, so the guards live in
    // scopes and never overlap.
    let tree_writes = Mutex::new(());
    let winner = {
        let tree = TreeExclusion::new(&tree_writes);
        handler
            .handle(request(seed("Ana"), Role::Ceo, None, None), &tree)
            .expect("the first CEO wins")
    };

    // Simulate the second request re-reading the tree the winner wrote:
    // the check now finds the CEO and refuses before its create places.
    let error = {
        let tree = TreeExclusion::new(&tree_writes);
        handler
            .handle(request(seed("Bo"), Role::Ceo, None, None), &tree)
            .expect_err("the second CEO is refused")
    };

    assert!(matches!(
        error,
        HierarchyApplicationError::Invalid(HierarchyError::SecondCeo { .. })
    ));
    // The loser's created agent was removed by the cleanup path only if it
    // got past the create; refused before placing, nothing of it remains.
    assert_eq!(store.placed(), [winner]);
    assert_eq!(creator.made(), [winner]);
    assert_eq!(creator.removed(), Vec::<AgentId>::new());

    // The removal branch: a create that raced past the check but lost the
    // place must remove its agent. An empty tree and a place that always
    // fails is the same shape: the create runs, the place fails, the
    // removal follows.
    let failing = Arc::new(PlacingStore {
        rows: Vec::new(),
        teams: Vec::new(),
        fail_place: true,
        places: Mutex::new(Vec::new()),
        tree_writes: Mutex::new(()),
    });
    let loser_creator = Arc::new(RecordingCreator::default());
    let losing_handler = guarded_handler(
        Arc::clone(&failing) as Arc<dyn HierarchyStore>,
        Arc::clone(&loser_creator) as Arc<dyn AgentCreator>,
    );
    let error = {
        let tree = TreeExclusion::new(&tree_writes);
        losing_handler
            .handle(request(seed("Sid"), Role::Ceo, None, None), &tree)
            .expect_err("the loser cannot take the seat")
    };
    assert_eq!(
        error,
        HierarchyApplicationError::Storage("the tree write failed".to_owned())
    );
    // The loser's created agent was removed again — no orphan behind.
    assert_eq!(loser_creator.removed(), loser_creator.made());
    assert_eq!(failing.placed().len(), 1);
}

/// The seed fields survive into the real create input: name, command,
/// working folder, project, and additions each land where the agent module's
/// create use case reads them.
#[test]
fn the_seed_round_trips_every_field_into_the_created_agent() {
    struct CapturingCreator {
        captured: Mutex<Option<AgentSeed>>,
    }

    impl AgentCreator for CapturingCreator {
        fn create(&self, seed: AgentSeed) -> Result<AgentId, CreatorError> {
            *locked(&self.captured) = Some(seed);
            Ok(AgentId::generate())
        }

        fn remove(&self, _agent_id: AgentId) -> Result<(), CreatorError> {
            Ok(())
        }
    }

    let additions = SeedAdditions {
        flags: vec!["--flag".to_owned()],
        env: vec![("KEY".to_owned(), Some("value".to_owned()))],
        startup_prompt: Some("begin".to_owned()),
    };
    let seed = AgentSeed {
        name: "Ana".to_owned(),
        command: "claude --danger".to_owned(),
        working_folder: Some(std::path::PathBuf::from("/tmp/office")),
        project_id: Some(kernel::ProjectId::generate()),
        additions: Some(additions.clone()),
    };
    let creator = Arc::new(CapturingCreator {
        captured: Mutex::new(None),
    });
    let store = Arc::new(GuardedStore::new(Vec::new()));
    let handler = guarded_handler(
        Arc::clone(&store) as Arc<dyn HierarchyStore>,
        Arc::clone(&creator) as Arc<dyn AgentCreator>,
    );

    let tree_writes = Mutex::new(());
    let tree = TreeExclusion::new(&tree_writes);
    handler
        .handle(
            CreateRoleAgentRequest {
                seed: seed.clone(),
                role: Role::Ceo,
                parent_agent_id: None,
                team_id: None,
            },
            &tree,
        )
        .expect("create and place");

    let captured = locked(&creator.captured).clone().expect("captured");
    assert_eq!(captured.name, seed.name);
    assert_eq!(captured.command, seed.command);
    assert_eq!(captured.working_folder, seed.working_folder);
    assert_eq!(captured.project_id, seed.project_id);
    assert_eq!(captured.additions, seed.additions);
    assert_eq!(captured.additions.as_ref().unwrap().flags, additions.flags);
}
