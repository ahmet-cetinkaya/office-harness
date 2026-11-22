// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::path::Path;
use std::sync::{Arc, Mutex};

use kernel::Timestamp;
use rusqlite::Connection;
use template_application::common::errors::TemplateApplicationError;
use template_application::common::interfaces::TemplateStore;
use template_domain::{
    AdditionsBody, AgentBody, EnvVariable, MemberEntry, NamePattern, TeamBody, Template,
    TemplateBody, TemplateId, TemplateName,
};
use template_infrastructure::persistence::{LEDGER_TABLE, SCRIPTS, TemplateRepository};

const NOW: Timestamp = Timestamp::from_unix_millis(1_700_000_000_000);

fn open_office(database: &Path) -> TemplateRepository {
    migrated(Connection::open(database).expect("open database"))
}

fn memory_office() -> TemplateRepository {
    migrated(Connection::open_in_memory().expect("open"))
}

fn migrated(connection: Connection) -> TemplateRepository {
    for script in SCRIPTS {
        connection.execute_batch(script).expect("migrate");
    }
    TemplateRepository::new(Arc::new(Mutex::new(connection)))
}

fn agent(pattern: &str, env_value: Option<&str>) -> AgentBody {
    AgentBody {
        name_pattern: NamePattern::new(pattern),
        command: "claude".to_owned(),
        working_folder: Some("/work".to_owned()),
        additions: AdditionsBody {
            flags: vec!["--verbose".to_owned()],
            env: vec![EnvVariable {
                name: "KEY".to_owned(),
                value: env_value.map(str::to_owned),
            }],
            startup_prompt: Some("go".to_owned()),
        },
        role: None,
        own_worktree: false,
    }
}

fn template(name: &str, body: TemplateBody) -> Template {
    Template::new(
        TemplateId::generate(),
        TemplateName::parse(name).expect("name"),
        body,
        NOW,
    )
    .expect("template")
}

fn agent_template(name: &str) -> Template {
    template(name, TemplateBody::Agent(agent("x-{n}", None)))
}

#[test]
fn the_ledger_is_named_for_the_module() {
    assert_eq!(LEDGER_TABLE, "template_schema_version");
    assert_eq!(SCRIPTS.len(), 1);
}

#[test]
fn templates_survive_closing_and_reopening_the_database() {
    let folder = tempfile::tempdir().expect("folder");
    let database = folder.path().join("office.db");
    let team = template(
        "Squad",
        TemplateBody::Team(TeamBody {
            lead: agent("{team}-lead", Some("")),
            members: vec![MemberEntry {
                entry: agent("{team}-member-{n}", None),
                count: 6,
            }],
        }),
    );
    let single = agent_template("Reviewer");
    {
        let office = open_office(&database);
        office.insert(&team).expect("team");
        office.insert(&single).expect("agent");
    }

    let reopened = TemplateRepository::new(Arc::new(Mutex::new(
        Connection::open(&database).expect("reopen"),
    )));

    let listed = reopened.list().expect("list");
    assert_eq!(listed, [single.clone(), team.clone()]);
    assert_eq!(reopened.find(team.id).expect("find"), Some(team));
}

#[test]
fn no_value_and_an_empty_value_stay_different_after_a_round_trip() {
    let office = memory_office();
    let saved = template(
        "Both",
        TemplateBody::Team(TeamBody {
            lead: agent("{team}-lead", None),
            members: vec![MemberEntry {
                entry: agent("m-{n}", Some("")),
                count: 1,
            }],
        }),
    );
    office.insert(&saved).expect("insert");

    let found = office.find(saved.id).expect("find").expect("row");

    assert_eq!(found, saved);
}

#[test]
fn a_name_is_unique_ignoring_case_on_insert_and_update() {
    let office = memory_office();
    office.insert(&agent_template("Reviewer")).expect("first");
    let other = agent_template("Other");
    office.insert(&other).expect("other");

    let duplicate = office.insert(&agent_template("REVIEWER")).expect_err("dup");
    let renamed = Template {
        name: TemplateName::parse("reviewer").expect("name"),
        ..other
    };
    let clash = office.update(&renamed).expect_err("clash");

    assert!(matches!(
        duplicate,
        TemplateApplicationError::NameTaken { .. }
    ));
    assert!(matches!(clash, TemplateApplicationError::NameTaken { .. }));
}

#[test]
fn update_replaces_a_row_and_delete_removes_it() {
    let office = memory_office();
    let saved = agent_template("Reviewer");
    office.insert(&saved).expect("insert");
    let changed = Template {
        body: TemplateBody::Agent(agent("changed-{n}", None)),
        ..saved.clone()
    };

    office.update(&changed).expect("update");
    let missing = office
        .update(&agent_template("Ghost"))
        .expect_err("missing");

    assert_eq!(office.find(saved.id).expect("find"), Some(changed));
    assert!(matches!(missing, TemplateApplicationError::NotFound(_)));
    assert!(office.delete(saved.id).expect("delete"));
    assert!(!office.delete(saved.id).expect("again"));
    assert_eq!(office.find(saved.id).expect("find"), None);
}

#[test]
fn insert_all_saves_every_template_or_none() {
    let office = memory_office();
    office.insert(&agent_template("Taken")).expect("seed");

    let failed = office.insert_all(&[agent_template("Fresh"), agent_template("taken")]);

    assert!(matches!(
        failed,
        Err(TemplateApplicationError::NameTaken { .. })
    ));
    assert_eq!(office.list().expect("list").len(), 1);
    office
        .insert_all(&[agent_template("A"), agent_template("B")])
        .expect("all");
    assert_eq!(office.list().expect("list").len(), 3);
}

#[test]
fn a_damaged_body_is_reported_as_a_storage_failure() {
    let connection = Connection::open_in_memory().expect("open");
    connection.execute_batch(SCRIPTS[0]).expect("migrate");
    connection
        .execute(
            "INSERT INTO template_templates (id, name, name_key, kind, body, updated_at) \
             VALUES (?1, 'Bad', 'bad', 'agent', '{\"body_version\":1}', 1)",
            [TemplateId::generate().to_string()],
        )
        .expect("row");
    let office = TemplateRepository::new(Arc::new(Mutex::new(connection)));

    let error = office.list().expect_err("damaged");

    assert!(matches!(error, TemplateApplicationError::Storage { .. }));
    assert!(error.to_string().contains("template body"));
}
