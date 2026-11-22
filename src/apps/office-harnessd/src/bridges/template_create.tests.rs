// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The create bridge against the real SQLite stores of a started office, so
//! a second take of the tree exclusion would hang a test instead of passing.

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError, mpsc};
use std::thread;
use std::time::Duration;

use agent_application::common::errors::PortError;
use agent_domain::entities::Project;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::DisplayName;
use hierarchy_application::common::errors::HierarchyApplicationError;
use hierarchy_application::common::ports::CreatorError;
use hierarchy_application::common::ports::CreatorErrorKind;
use kernel::Timestamp;
use tempfile::TempDir;
use template_application::common::errors::TemplateApplicationError;
use template_domain::{
    AdditionsBody, EnvVariable, MemberEntry, NamePattern, TeamBody, TemplateName,
};
use wire::{ErrorCode, Request};

use super::*;
use crate::bootstrap::{self, LoginSetup, Services};
use crate::config::Config;

/// A creation that does not finish in this time has deadlocked.
const DEADLINE: Duration = Duration::from_secs(60);
const TEAM_NAME: &str = "Core";

struct Office {
    services: Services,
    folder: TempDir,
    _data: TempDir,
}

fn office() -> Office {
    let data = tempfile::tempdir().expect("data folder");
    let config = Config {
        data_folder: data.path().join("office"),
    };
    let log = bootstrap::open_log(&config).expect("log");
    let services = bootstrap::start_with(&config, &log, LoginSetup::detect()).expect("start");
    Office {
        services,
        folder: tempfile::tempdir().expect("working folder"),
        _data: data,
    }
}

fn within_deadline<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
        .recv_timeout(DEADLINE)
        .expect("the creation finished; a hang is a deadlock on the tree exclusion")
}

fn agent_body(pattern: &str, working_folder: Option<&Path>) -> AgentBody {
    AgentBody {
        name_pattern: NamePattern::new(pattern),
        command: "claude".to_owned(),
        working_folder: working_folder.map(|folder| folder.display().to_string()),
        additions: AdditionsBody::default(),
        role: None,
    }
}

fn with_role(mut body: AgentBody, role: TemplateRole) -> AgentBody {
    body.role = Some(role);
    body
}

fn team_body(folder: &Path, member_count: u32) -> TeamBody {
    TeamBody {
        lead: agent_body("{team}-lead", Some(folder)),
        members: vec![MemberEntry {
            entry: agent_body("{team}-member-{n}", Some(folder)),
            count: member_count,
        }],
    }
}

impl Office {
    fn save(&self, name: &str, body: TemplateBody) -> TemplateId {
        let template = Template::new(
            TemplateId::generate(),
            TemplateName::parse(name).expect("name"),
            body,
            Timestamp::from_unix_millis(1),
        )
        .expect("template");
        self.services
            .template
            .templates
            .insert(&template)
            .expect("insert");
        template.id
    }

    fn agent_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .services
            .agent
            .agent_reader
            .handle()
            .expect("agents")
            .iter()
            .map(|agent| agent.name.as_str().to_owned())
            .collect();
        names.sort();
        names
    }

    fn team_count(&self) -> usize {
        self.services
            .hierarchy
            .store
            .snapshot()
            .expect("tree")
            .teams
            .len()
    }

    fn try_create_agent(
        &self,
        id: TemplateId,
        place: Option<DirectPlace>,
        project_id: Option<ProjectId>,
    ) -> Result<CreatedAgent, TemplateCreateError> {
        let bridge = Arc::clone(&self.services.template.create);
        within_deadline(move || bridge.create_agent(id, place, project_id))
    }

    fn try_create_team(
        &self,
        id: TemplateId,
        team_name: &'static str,
        parent: AgentId,
        project_id: Option<ProjectId>,
    ) -> Result<TeamCreation, TemplateCreateError> {
        let bridge = Arc::clone(&self.services.template.create);
        within_deadline(move || bridge.create_team(id, team_name, parent, project_id))
    }

    fn create_ceo(&self) -> AgentId {
        let body = with_role(
            agent_body("ceo", Some(self.folder.path())),
            TemplateRole::Ceo,
        );
        let id = self.save("CEO", TemplateBody::Agent(body));
        self.try_create_agent(id, None, None).expect("ceo").id
    }

    fn create_team(&self, parent: AgentId, members: u32) -> TeamCreation {
        let id = self.save(
            &format!("Team {members}"),
            TemplateBody::Team(team_body(self.folder.path(), members)),
        );
        self.try_create_team(id, TEAM_NAME, parent, None)
            .expect("team")
    }

    fn bridge_with(&self, creator: Arc<dyn AgentCreator>) -> Arc<TemplateCreateBridge> {
        self.bridge_over(Arc::clone(&self.services.template.templates), creator)
    }

    fn bridge_over(
        &self,
        templates: Arc<dyn TemplateStore>,
        creator: Arc<dyn AgentCreator>,
    ) -> Arc<TemplateCreateBridge> {
        self.bridge_of(
            templates,
            Arc::clone(&self.services.hierarchy.store),
            creator,
            Arc::clone(&self.services.agent_bridge_store),
        )
    }

    fn bridge_from(
        &self,
        store: Arc<dyn HierarchyStore>,
        agents: Arc<dyn AgentStore>,
    ) -> Arc<TemplateCreateBridge> {
        self.bridge_of(
            Arc::clone(&self.services.template.templates),
            store,
            Arc::clone(&self.services.hierarchy.creator),
            agents,
        )
    }

    fn bridge_of(
        &self,
        templates: Arc<dyn TemplateStore>,
        store: Arc<dyn HierarchyStore>,
        creator: Arc<dyn AgentCreator>,
        agents: Arc<dyn AgentStore>,
    ) -> Arc<TemplateCreateBridge> {
        Arc::new(TemplateCreateBridge::new(
            templates,
            store,
            creator,
            agents,
            Arc::clone(&self.services.template.projects),
            self.services.storage_alerts.clone(),
        ))
    }

    fn actions_of(&self, agent_id: AgentId) -> Vec<(String, String)> {
        let request = Request {
            id: 1,
            kind: "agent.actions".to_owned(),
            fields: serde_json::from_value(serde_json::json!({"agent_id": agent_id.to_string()}))
                .expect("fields"),
        };
        let (response, _) = self.services.agent.audit.handle(&request);
        let wire::Response::Success { result, .. } = response else {
            panic!("agent.actions failed");
        };
        result
            .as_array()
            .expect("entries")
            .iter()
            .map(|entry| (text(entry, "action"), text(entry, "actor")))
            .collect()
    }
}

fn agent_of(created: &CreatedAgent) -> &Agent {
    created.agent.as_ref().expect("the agent was read back")
}

fn text(entry: &serde_json::Value, key: &str) -> String {
    entry[key].as_str().expect("text").to_owned()
}

/// Wraps the real creator and fails on request: the create at one position,
/// or the removal of the agent that one position made.
struct FaultyCreator {
    inner: Arc<dyn AgentCreator>,
    fail_create_at: Option<usize>,
    fail_remove_of: Option<usize>,
    creates: AtomicUsize,
    made: Mutex<Vec<AgentId>>,
}

impl FaultyCreator {
    fn new(
        inner: Arc<dyn AgentCreator>,
        fail_create_at: Option<usize>,
        fail_remove_of: Option<usize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner,
            fail_create_at,
            fail_remove_of,
            creates: AtomicUsize::new(0),
            made: Mutex::new(Vec::new()),
        })
    }

    fn made(&self) -> Vec<AgentId> {
        self.made
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

fn injected_failure(what: &str) -> CreatorError {
    CreatorError {
        kind: CreatorErrorKind::Internal,
        message: format!("injected {what} failure"),
    }
}

impl AgentCreator for FaultyCreator {
    fn check(&self, seed: &AgentSeed) -> Vec<String> {
        self.inner.check(seed)
    }

    fn create(&self, seed: AgentSeed) -> Result<AgentId, CreatorError> {
        let position = self.creates.fetch_add(1, Ordering::SeqCst);
        if self.fail_create_at == Some(position) {
            return Err(injected_failure("create"));
        }
        let id = self.inner.create(seed)?;
        self.made
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(id);
        Ok(id)
    }

    fn remove(&self, agent_id: AgentId) -> Result<(), CreatorError> {
        let position = self.made().iter().position(|made| *made == agent_id);
        if position.is_some() && position == self.fail_remove_of {
            return Err(injected_failure("remove"));
        }
        self.inner.remove(agent_id)
    }
}

/// A template store that answers every find with one template, which the
/// real store would refuse to load (a damaged or hand-edited row).
struct FixedTemplate(Template);

impl TemplateStore for FixedTemplate {
    fn list(&self) -> Result<Vec<Template>, TemplateApplicationError> {
        Ok(vec![self.0.clone()])
    }

    fn find(&self, _id: TemplateId) -> Result<Option<Template>, TemplateApplicationError> {
        Ok(Some(self.0.clone()))
    }

    fn insert(&self, _template: &Template) -> Result<(), TemplateApplicationError> {
        unreachable!("the bridge never writes templates")
    }

    fn update(&self, _template: &Template) -> Result<(), TemplateApplicationError> {
        unreachable!("the bridge never writes templates")
    }

    fn delete(&self, _id: TemplateId) -> Result<bool, TemplateApplicationError> {
        unreachable!("the bridge never writes templates")
    }

    fn insert_all(&self, _templates: &[Template]) -> Result<(), TemplateApplicationError> {
        unreachable!("the bridge never writes templates")
    }
}

fn unvalidated(body: TemplateBody) -> Template {
    Template {
        id: TemplateId::generate(),
        name: TemplateName::parse("Damaged").expect("name"),
        body,
        updated_at: Timestamp::from_unix_millis(1),
    }
}

#[test]
fn a_team_creation_that_fails_at_any_agent_leaves_nothing_behind() {
    // Positions: 0 is the lead, 1 the first member, 3 the last of three.
    for position in [0, 1, 3] {
        let office = office();
        let ceo = office.create_ceo();
        let id = office.save(
            "Team",
            TemplateBody::Team(team_body(office.folder.path(), 3)),
        );
        let creator = FaultyCreator::new(
            Arc::clone(&office.services.hierarchy.creator),
            Some(position),
            None,
        );
        let bridge = office.bridge_with(creator.clone());

        let failure = within_deadline(move || bridge.create_team(id, TEAM_NAME, ceo, None))
            .expect_err("the injected failure stops the creation");

        assert_eq!(failure.code, ErrorCode::Internal, "position {position}");
        assert!(failure.message.contains("injected create failure"));
        assert_eq!(office.agent_names(), vec!["ceo"], "position {position}");
        assert_eq!(office.team_count(), 0, "position {position}");
    }
}

#[test]
fn a_removal_that_fails_in_the_undo_leaves_only_that_agent_and_is_named() {
    let office = office();
    let ceo = office.create_ceo();
    let id = office.save(
        "Team",
        TemplateBody::Team(team_body(office.folder.path(), 3)),
    );
    // The last member fails to create; the first member cannot be removed.
    let creator = FaultyCreator::new(
        Arc::clone(&office.services.hierarchy.creator),
        Some(3),
        Some(1),
    );
    let bridge = office.bridge_with(creator.clone());

    let failure = within_deadline(move || bridge.create_team(id, TEAM_NAME, ceo, None))
        .expect_err("the injected failure stops the creation");

    assert!(
        failure.message.starts_with("injected create failure"),
        "the first reason leads: {}",
        failure.message
    );
    assert!(failure.message.contains("Core-member-1"));
    assert!(failure.message.contains("injected remove failure"));
    assert_eq!(office.agent_names(), vec!["Core-member-1", "ceo"]);
    assert_eq!(office.team_count(), 0);
}

#[test]
fn each_rolled_back_agent_has_a_create_and_a_delete_entry() {
    let office = office();
    let ceo = office.create_ceo();
    let id = office.save(
        "Team",
        TemplateBody::Team(team_body(office.folder.path(), 2)),
    );
    let creator = FaultyCreator::new(
        Arc::clone(&office.services.hierarchy.creator),
        Some(2),
        None,
    );
    let bridge = office.bridge_with(creator.clone());

    let _ = within_deadline(move || bridge.create_team(id, TEAM_NAME, ceo, None));

    let rolled_back = creator.made();
    assert_eq!(rolled_back.len(), 2);
    for agent_id in rolled_back {
        let actions = office.actions_of(agent_id);
        let kinds: Vec<&str> = actions.iter().map(|(action, _)| action.as_str()).collect();
        assert_eq!(kinds, vec!["create", "delete"]);
    }
}

#[test]
fn every_pre_check_failure_is_named_in_one_answer_and_nothing_is_created() {
    let office = office();
    let ceo = office.create_ceo();
    let folder = office.folder.path();
    let long_team_name = "T".repeat(60);
    let broken = |command: &str, pattern: &str| AgentBody {
        command: command.to_owned(),
        ..agent_body(pattern, Some(folder))
    };
    let mut reserved = agent_body("reserved", Some(folder));
    reserved.additions.env = vec![EnvVariable {
        name: "OFFICE_HARNESS_TOKEN".to_owned(),
        value: Some("x".to_owned()),
    }];
    let body = TeamBody {
        lead: agent_body("{team}-lead-with-a-long-tail", Some(folder)),
        members: vec![
            MemberEntry {
                entry: broken("claude \"unclosed", "unsplittable"),
                count: 1,
            },
            MemberEntry {
                entry: reserved,
                count: 1,
            },
        ],
    };
    let template = unvalidated(TemplateBody::Team(body));
    let id = template.id;
    let bridge = office.bridge_over(
        Arc::new(FixedTemplate(template)),
        Arc::clone(&office.services.hierarchy.creator),
    );

    let failure = bridge
        .create_team(
            id,
            &long_team_name,
            ceo,
            Some(kernel::ProjectId::generate()),
        )
        .expect_err("the plan is refused");

    assert_eq!(failure.code, ErrorCode::Invalid);
    for reason in [
        "at most 64 characters",
        "never closed",
        "OFFICE_HARNESS_",
        "was not found",
    ] {
        assert!(
            failure.message.contains(reason),
            "missing {reason:?} in {}",
            failure.message
        );
    }
    assert_eq!(office.agent_names(), vec!["ceo"]);
    assert_eq!(office.team_count(), 0);
}

#[test]
fn a_bad_team_name_is_a_reason() {
    let office = office();
    let ceo = office.create_ceo();
    let id = office.save(
        "Team",
        TemplateBody::Team(team_body(office.folder.path(), 1)),
    );

    let failure = office
        .try_create_team(id, "   ", ceo, None)
        .expect_err("a blank team name is refused");

    assert_eq!(failure.code, ErrorCode::Invalid);
    assert!(failure.message.contains("team name must not be empty"));
}

#[test]
fn a_taken_team_name_alone_is_a_conflict_and_creates_nothing() {
    let office = office();
    let ceo = office.create_ceo();
    office.create_team(ceo, 1);
    let before = office.agent_names();
    let id = office.save(
        "Again",
        TemplateBody::Team(team_body(office.folder.path(), 1)),
    );

    let failure = office
        .try_create_team(id, "core", ceo, None)
        .expect_err("the team name is taken");

    assert_eq!(failure.code, ErrorCode::Conflict);
    assert!(
        failure
            .message
            .contains("team name \"core\" is already used")
    );
    assert_eq!(office.agent_names(), before);
}

#[test]
fn a_taken_team_name_joins_the_other_reasons() {
    let office = office();
    let ceo = office.create_ceo();
    office.create_team(ceo, 1);
    let mut body = team_body(office.folder.path(), 1);
    body.lead.command = "claude \"unclosed".to_owned();
    let template = unvalidated(TemplateBody::Team(body));
    let id = template.id;
    let bridge = office.bridge_over(
        Arc::new(FixedTemplate(template)),
        Arc::clone(&office.services.hierarchy.creator),
    );

    let failure = bridge
        .create_team(id, TEAM_NAME, ceo, None)
        .expect_err("two reasons");

    assert_eq!(failure.code, ErrorCode::Invalid);
    assert!(failure.message.contains("is already used"));
    assert!(failure.message.contains("never closed"));
}

#[test]
fn a_missing_working_folder_is_named_before_anything_is_created() {
    let office = office();
    let missing = office.folder.path().join("missing-folder");
    let id = office.save(
        "Reviewer",
        TemplateBody::Agent(agent_body("reviewer-{n}", Some(&missing))),
    );

    let failure = office
        .try_create_agent(id, None, None)
        .expect_err("the folder is missing");

    assert_eq!(failure.code, ErrorCode::Invalid);
    assert!(failure.message.contains("missing-folder"));
    assert_eq!(office.agent_names(), Vec::<String>::new());
}

#[test]
fn no_folder_and_no_project_needs_a_working_folder() {
    let office = office();
    let id = office.save(
        "Reviewer",
        TemplateBody::Agent(agent_body("reviewer-{n}", None)),
    );

    let failure = office
        .try_create_agent(id, None, None)
        .expect_err("no folder");

    assert!(failure.message.contains("a working folder is needed"));
}

#[test]
fn a_project_gives_the_working_folder_when_the_template_has_none() {
    let office = office();
    let project = Project::new(
        kernel::ProjectId::generate(),
        DisplayName::parse("project", "api").expect("name"),
        office.folder.path().to_path_buf(),
    )
    .expect("project");
    office
        .services
        .template
        .projects
        .insert(&project)
        .expect("insert");
    let id = office.save(
        "Reviewer",
        TemplateBody::Agent(agent_body("reviewer-{n}", None)),
    );

    let created = office
        .try_create_agent(id, None, Some(project.id))
        .expect("created");

    assert_eq!(agent_of(&created).project_id, Some(project.id));
    assert_eq!(agent_of(&created).working_folder, None);
}

#[test]
fn a_team_pattern_on_a_direct_creation_without_a_team_names_the_pattern() {
    let office = office();
    let id = office.save(
        "Helper",
        TemplateBody::Agent(agent_body("{team}-helper", Some(office.folder.path()))),
    );

    let failure = office
        .try_create_agent(id, None, None)
        .expect_err("no team");

    assert_eq!(failure.code, ErrorCode::Invalid);
    assert!(failure.message.contains("\"{team}-helper\""));
}

#[test]
fn a_value_less_environment_entry_keeps_no_value_in_the_stored_additions() {
    let office = office();
    let mut body = agent_body("worker-{n}", Some(office.folder.path()));
    body.additions.env = vec![
        EnvVariable {
            name: "TOKEN".to_owned(),
            value: None,
        },
        EnvVariable {
            name: "MODE".to_owned(),
            value: Some("fast".to_owned()),
        },
    ];
    let id = office.save("Worker", TemplateBody::Agent(body));

    let created = office.try_create_agent(id, None, None).expect("created");

    let view = office
        .services
        .agent
        .handlers
        .use_cases()
        .get_office_additions
        .handle(created.id)
        .expect("additions");
    let stored: Vec<(&str, Option<&str>)> = view
        .record
        .additions
        .env()
        .iter()
        .map(|entry| (entry.name(), entry.value()))
        .collect();
    assert_eq!(stored, vec![("TOKEN", None), ("MODE", Some("fast"))]);
}

#[test]
fn a_second_ceo_is_refused_with_the_tree_rule_text() {
    let office = office();
    office.create_ceo();
    let id = office.save(
        "Another CEO",
        TemplateBody::Agent(with_role(
            agent_body("ceo-two", Some(office.folder.path())),
            TemplateRole::Ceo,
        )),
    );

    let failure = office
        .try_create_agent(id, None, None)
        .expect_err("a CEO exists");

    assert_eq!(failure.code, ErrorCode::Refused);
    assert!(
        failure
            .message
            .contains("the office already has a CEO: ceo")
    );
    assert_eq!(office.agent_names(), vec!["ceo"]);
}

#[test]
fn a_team_lead_entry_with_the_ceo_role_is_created_as_a_lead() {
    let office = office();
    let ceo = office.create_ceo();
    let mut body = team_body(office.folder.path(), 1);
    body.lead.role = Some(TemplateRole::Ceo);
    let id = office.save("Team", TemplateBody::Team(body));

    let created = office
        .try_create_team(id, TEAM_NAME, ceo, None)
        .expect("team");

    let roles: Vec<Option<Role>> = created.agents.iter().map(|agent| agent.role).collect();
    assert_eq!(roles, vec![Some(Role::Lead), Some(Role::Member)]);
    let rows = office
        .services
        .hierarchy
        .store
        .snapshot()
        .expect("tree")
        .rows;
    let lead = rows
        .iter()
        .find(|row| row.agent_id == created.agents[0].id)
        .expect("lead row");
    assert_eq!(lead.parent_agent_id, Some(ceo));
}

#[test]
fn a_team_creation_names_every_agent_from_its_pattern() {
    let office = office();
    let ceo = office.create_ceo();

    let created = office.create_team(ceo, 3);

    let names: Vec<&str> = created
        .agents
        .iter()
        .map(|created| agent_of(created).name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "Core-lead",
            "Core-member-1",
            "Core-member-2",
            "Core-member-3"
        ]
    );
    assert_eq!(office.team_count(), 1);
}

#[test]
fn a_member_template_is_placed_under_a_lead_with_the_team_name_filled_in() {
    let office = office();
    let ceo = office.create_ceo();
    let team = office.create_team(ceo, 1);
    let body = with_role(
        agent_body("{team}-extra-{n}", Some(office.folder.path())),
        TemplateRole::Member,
    );
    let id = office.save("Extra", TemplateBody::Agent(body));
    let place = DirectPlace {
        parent_agent_id: team.agents[0].id,
        team_id: team.team.id(),
    };

    let created = office
        .try_create_agent(id, Some(place), None)
        .expect("member");

    assert_eq!(agent_of(&created).name.as_str(), "Core-extra-1");
    assert_eq!(created.role, Some(Role::Member));
    assert_eq!(created.team_name.as_deref(), Some(TEAM_NAME));
}

#[test]
fn a_place_for_a_template_with_no_role_is_a_reason() {
    let office = office();
    let ceo = office.create_ceo();
    let team = office.create_team(ceo, 1);
    let id = office.save(
        "Plain",
        TemplateBody::Agent(agent_body("plain-{n}", Some(office.folder.path()))),
    );
    let place = DirectPlace {
        parent_agent_id: ceo,
        team_id: team.team.id(),
    };

    let failure = office
        .try_create_agent(id, Some(place), None)
        .expect_err("no role, no place");

    assert_eq!(failure.code, ErrorCode::Invalid);
    assert!(failure.message.contains("place must be absent"));
}

#[test]
fn two_concurrent_creations_from_one_template_get_different_names() {
    let office = office();
    let id = office.save(
        "Reviewer",
        TemplateBody::Agent(agent_body("reviewer-{n}", Some(office.folder.path()))),
    );
    let bridge = Arc::clone(&office.services.template.create);

    let names: HashSet<String> = within_deadline(move || {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let bridge = Arc::clone(&bridge);
                thread::spawn(move || bridge.create_agent(id, None, None).expect("created"))
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| {
                agent_of(&handle.join().expect("thread"))
                    .name
                    .as_str()
                    .to_owned()
            })
            .collect()
    });

    assert_eq!(
        names,
        HashSet::from(["reviewer-1".to_owned(), "reviewer-2".to_owned()])
    );
}

#[test]
fn each_created_agent_has_a_create_record_with_the_user_as_actor() {
    let office = office();
    let ceo = office.create_ceo();

    let created = office.create_team(ceo, 2);

    for agent in &created.agents {
        assert_eq!(
            office.actions_of(agent.id),
            vec![("create".to_owned(), "user".to_owned())]
        );
    }
}

/// Wraps the real tree store and fails one write step on request: the n-th
/// `place_agent` call (counting from 1), or the team save.
struct FaultyStore {
    inner: Arc<dyn HierarchyStore>,
    fail_place_at: Option<usize>,
    fail_team_save: bool,
    places: AtomicUsize,
}

impl FaultyStore {
    fn failing_place_at(inner: Arc<dyn HierarchyStore>, position: usize) -> Arc<Self> {
        Arc::new(Self {
            inner,
            fail_place_at: Some(position),
            fail_team_save: false,
            places: AtomicUsize::new(0),
        })
    }

    fn failing_team_save(inner: Arc<dyn HierarchyStore>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            fail_place_at: None,
            fail_team_save: true,
            places: AtomicUsize::new(0),
        })
    }
}

fn injected_store_failure(what: &str) -> HierarchyApplicationError {
    HierarchyApplicationError::Storage(format!("injected {what} failure"))
}

impl HierarchyStore for FaultyStore {
    fn snapshot(&self) -> Result<TreeSnapshot, HierarchyApplicationError> {
        self.inner.snapshot()
    }

    fn tree_exclusion(&self) -> TreeExclusion<'_> {
        self.inner.tree_exclusion()
    }

    fn team_name_taken(&self, key: &str) -> Result<bool, HierarchyApplicationError> {
        self.inner.team_name_taken(key)
    }

    fn save_team_and_place_lead_held(
        &self,
        exclusion: &TreeExclusion<'_>,
        team: &Team,
        parent_agent_id: Option<AgentId>,
    ) -> Result<(), HierarchyApplicationError> {
        if self.fail_team_save {
            return Err(injected_store_failure("team save"));
        }
        self.inner
            .save_team_and_place_lead_held(exclusion, team, parent_agent_id)
    }

    fn delete_team_and_unplace_held(
        &self,
        exclusion: &TreeExclusion<'_>,
        team_id: TeamId,
    ) -> Result<(), HierarchyApplicationError> {
        self.inner.delete_team_and_unplace_held(exclusion, team_id)
    }

    fn place_agent(
        &self,
        agent_id: AgentId,
        role: Option<Role>,
        parent_agent_id: Option<AgentId>,
        team_id: Option<TeamId>,
    ) -> Result<(), HierarchyApplicationError> {
        let position = self.places.fetch_add(1, Ordering::SeqCst) + 1;
        if self.fail_place_at == Some(position) {
            return Err(injected_store_failure("place"));
        }
        self.inner
            .place_agent(agent_id, role, parent_agent_id, team_id)
    }

    fn team_name_of(&self, team_id: TeamId) -> Result<Option<TeamName>, HierarchyApplicationError> {
        self.inner.team_name_of(team_id)
    }
}

/// Reads fail, as a store does when the disk fails right after a write.
struct UnreadableAgents(Arc<dyn AgentStore>);

impl AgentStore for UnreadableAgents {
    fn insert(&self, agent: &Agent) -> Result<(), PortError> {
        self.0.insert(agent)
    }

    fn find(&self, _id: AgentId) -> Result<Option<Agent>, PortError> {
        Err(PortError::Storage {
            message: "injected read failure".to_owned(),
            is_disk_failure: true,
        })
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        self.0.list()
    }

    fn update_status(&self, id: AgentId, status: AgentStatus) -> Result<(), PortError> {
        self.0.update_status(id, status)
    }

    fn delete(&self, id: AgentId) -> Result<(), PortError> {
        self.0.delete(id)
    }
}

impl Office {
    fn bridge_over_store(&self, store: Arc<dyn HierarchyStore>) -> Arc<TemplateCreateBridge> {
        self.bridge_from(store, Arc::clone(&self.services.agent_bridge_store))
    }
}

#[test]
fn a_team_creation_whose_store_step_fails_removes_every_agent_and_the_team() {
    // The lead is placed with the team save; the places counted here are the
    // members', so 1 is the first member and 3 the last of three.
    for fail_place_at in [1, 3] {
        let office = office();
        let ceo = office.create_ceo();
        let id = office.save(
            "Team",
            TemplateBody::Team(team_body(office.folder.path(), 3)),
        );
        let store = FaultyStore::failing_place_at(
            Arc::clone(&office.services.hierarchy.store),
            fail_place_at,
        );
        let bridge = office.bridge_over_store(store);

        let failure = within_deadline(move || bridge.create_team(id, TEAM_NAME, ceo, None))
            .expect_err("the injected failure stops the creation");

        assert_eq!(failure.code, ErrorCode::Internal);
        assert!(failure.message.contains("injected place failure"));
        assert_eq!(office.agent_names(), vec!["ceo"], "place {fail_place_at}");
        assert_eq!(office.team_count(), 0, "place {fail_place_at}");
    }
}

#[test]
fn a_team_creation_whose_team_save_fails_removes_the_lead() {
    let office = office();
    let ceo = office.create_ceo();
    let id = office.save(
        "Team",
        TemplateBody::Team(team_body(office.folder.path(), 2)),
    );
    let store = FaultyStore::failing_team_save(Arc::clone(&office.services.hierarchy.store));
    let bridge = office.bridge_over_store(store);

    let failure = within_deadline(move || bridge.create_team(id, TEAM_NAME, ceo, None))
        .expect_err("the injected failure stops the creation");

    assert!(failure.message.contains("injected team save failure"));
    assert_eq!(office.agent_names(), vec!["ceo"]);
    assert_eq!(office.team_count(), 0);
}

#[test]
fn a_direct_creation_whose_placement_fails_removes_the_agent() {
    let office = office();
    let ceo = office.create_ceo();
    let team = office.create_team(ceo, 1);
    let id = office.save(
        "Extra",
        TemplateBody::Agent(with_role(
            agent_body("{team}-extra-{n}", Some(office.folder.path())),
            TemplateRole::Member,
        )),
    );
    let place = DirectPlace {
        parent_agent_id: team.agents[0].id,
        team_id: team.team.id(),
    };
    let names_before = office.agent_names();
    let store = FaultyStore::failing_place_at(Arc::clone(&office.services.hierarchy.store), 1);
    let bridge = office.bridge_over_store(store);

    let failure = within_deadline(move || bridge.create_agent(id, Some(place), None))
        .expect_err("the injected failure stops the creation");

    assert!(failure.message.contains("injected place failure"));
    assert_eq!(office.agent_names(), names_before);
}

#[test]
fn a_failed_read_after_the_writes_still_answers_success_and_raises_an_alert() {
    let office = office();
    let ceo = office.create_ceo();
    let id = office.save(
        "Team",
        TemplateBody::Team(team_body(office.folder.path(), 2)),
    );
    let mut alerts = office.services.storage_alerts.subscribe();
    let bridge = office.bridge_from(
        Arc::clone(&office.services.hierarchy.store),
        Arc::new(UnreadableAgents(Arc::clone(
            &office.services.agent_bridge_store,
        ))),
    );

    let created = within_deadline(move || bridge.create_team(id, TEAM_NAME, ceo, None))
        .expect("the writes succeeded, so the creation succeeded");

    assert_eq!(created.agents.len(), 3);
    assert!(created.agents.iter().all(|agent| agent.agent.is_none()));
    assert_eq!(created.agents[0].name, "Core-lead");
    assert_eq!(office.team_count(), 1);
    assert_eq!(office.agent_names().len(), 4);
    assert!(
        alerts
            .try_recv()
            .expect("an alert")
            .message
            .contains("injected read failure")
    );
}

#[test]
fn an_undo_that_leaves_an_agent_behind_raises_an_alert() {
    let office = office();
    let ceo = office.create_ceo();
    let id = office.save(
        "Team",
        TemplateBody::Team(team_body(office.folder.path(), 3)),
    );
    let creator = FaultyCreator::new(
        Arc::clone(&office.services.hierarchy.creator),
        Some(3),
        Some(1),
    );
    let mut alerts = office.services.storage_alerts.subscribe();
    let bridge = office.bridge_with(creator);

    let _ = within_deadline(move || bridge.create_team(id, TEAM_NAME, ceo, None));

    let alert = alerts.try_recv().expect("an alert");
    assert!(alert.message.contains("Core-member-1"));
    assert!(alert.message.contains("injected remove failure"));
}
