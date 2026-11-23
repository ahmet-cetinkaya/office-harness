// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use hierarchy_application::common::interfaces::TreeExclusion;
use wire::ErrorCode;

fn seed(name: &str) -> RoleSeed {
    RoleSeed {
        name: name.to_owned(),
        command: "fake-cli".to_owned(),
        working_folder: None,
        project_id: None,
        additions: None,
        worktree: None,
        temp: None,
    }
}

#[test]
fn a_temp_seed_carries_its_objective_to_the_hierarchy_seed() {
    let mut temp_seed = seed("Ana");
    temp_seed.temp = Some(NewTemp {
        objective: "fix it".to_owned(),
        creator: kernel::Actor::User,
    });

    let carried = seed_of(&temp_seed).expect("seed");

    assert_eq!(
        carried.temp.map(|temp| temp.objective),
        Some("fix it".to_owned())
    );
}

#[test]
fn the_place_reads_every_kind_and_names_the_broken_field() {
    let ceo = place_of("ceo", None, None);
    assert!(ceo.is_ok());

    let unknown = place_of("boss", None, None).expect_err("an unknown kind is refused");
    assert!(unknown.contains("role.kind"), "{unknown}");

    let bad_parent = place_of("member", Some("not-a-uuid".to_owned()), None)
        .expect_err("a broken parent id is refused");
    assert_eq!(bad_parent, "role.parent_agent_id is not an agent id");

    let bad_team = place_of("lead", None, Some("not-a-uuid".to_owned()))
        .expect_err("a broken team id is refused");
    assert_eq!(bad_team, "role.team_id is not a team id");
}

#[test]
fn a_broken_project_id_is_refused_before_any_handler_runs() {
    let bridge = RoleRequestsBridge::new(
        CreateRoleAgentHandler::new(Arc::new(FailingStore), Arc::new(RefusingCreator)),
        Arc::new(FailingStore),
    );

    let mut seeded = seed("Ana");
    seeded.project_id = Some("not-a-uuid".to_owned());
    let outcome = bridge.create_role_agent(&seeded, "ceo", None, None);

    assert_eq!(
        outcome,
        RoleResult::Failed {
            code: ErrorCode::Invalid,
            message: "project_id is not a valid identifier: not-a-uuid".to_owned(),
        }
    );
}

/// An agent-module `NotFound` or `Conflict` rides to the wire bare (#40
/// LOW): the kind keeps the code `agent.create` answers with and the
/// message stays the agent module's own, without a hierarchy prefix.
#[test]
fn an_agent_module_not_found_or_conflict_passes_through_without_a_prefix() {
    let cases = [
        (
            HierarchyApplicationError::NotFoundInput("the project is gone".to_owned()),
            ErrorCode::NotFound,
        ),
        (
            HierarchyApplicationError::ConflictInput("an agent named Ana exists".to_owned()),
            ErrorCode::Conflict,
        ),
    ];
    for (error, code) in cases {
        let outcome = RoleResult::failed(wire_code(&error), error.to_string());

        assert_eq!(
            outcome,
            RoleResult::Failed {
                code,
                message: error_message(&error)
            }
        );
    }
}

/// The message of an error, for comparing against the bridge's answer.
fn error_message(error: &HierarchyApplicationError) -> String {
    match error {
        HierarchyApplicationError::NotFoundInput(message)
        | HierarchyApplicationError::ConflictInput(message) => message.clone(),
        other => other.to_string(),
    }
}

/// The store these parse-failure tests never reach: the refusal happens
/// before the handler or the store runs.
struct FailingStore;

impl HierarchyStore for FailingStore {
    fn snapshot(
        &self,
    ) -> Result<hierarchy_application::common::interfaces::TreeSnapshot, HierarchyApplicationError>
    {
        Err(HierarchyApplicationError::Storage("no store".to_owned()))
    }

    fn tree_exclusion(&self) -> TreeExclusion<'_> {
        unreachable!("the parse failures never take the exclusion")
    }

    fn team_name_taken(&self, _key: &str) -> Result<bool, HierarchyApplicationError> {
        unreachable!()
    }

    fn save_team_and_place_lead_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
        _team: &hierarchy_domain::Team,
        _parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        unreachable!()
    }

    fn delete_team_and_unplace_held(
        &self,
        _exclusion: &TreeExclusion<'_>,
        _team_id: hierarchy_domain::TeamId,
    ) -> Result<(), HierarchyApplicationError> {
        unreachable!()
    }

    fn place_agent(
        &self,
        _agent_id: AgentId,
        _role: Option<hierarchy_domain::Role>,
        _parent_agent_id: Option<AgentId>,
        _team_id: Option<hierarchy_domain::TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        unreachable!()
    }

    fn team_name_of(
        &self,
        _team_id: hierarchy_domain::TeamId,
    ) -> Result<Option<hierarchy_domain::TeamName>, HierarchyApplicationError> {
        unreachable!()
    }
}

/// The creator these parse-failure tests never reach.
struct RefusingCreator;

impl AgentCreator for RefusingCreator {
    fn create(&self, _seed: AgentSeed) -> Result<AgentId, CreatorError> {
        unreachable!("the parse failures never create")
    }

    fn remove(&self, _agent_id: AgentId) -> Result<(), CreatorError> {
        unreachable!()
    }
}
