// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! Templates (0009), through a real client against a real server: saving,
//! creating agents and teams from them, moving them between offices through
//! a file, and the events that other clients receive.

mod support;

use std::time::Duration;

use rusqlite::Connection;
use serde_json::{Value, json};
use support::entries_under;
use support::office_client::{Client, Office, error_text};
use tokio::time::timeout;

const EVENT_WAIT: Duration = Duration::from_secs(20);
const MAX_FILE_BYTES: usize = 256 * 1024;
const SECRET_VALUE: &str = "s3cret-value";
const MEMBER_COUNT: u64 = 6;

/// Sends a request that must succeed and returns its result.
async fn ok(client: &mut Client, kind: &str, fields: Value) -> Value {
    let answer = client.ask(kind, fields).await;
    assert_eq!(answer["ok"], true, "{kind}: {answer}");
    answer["result"].clone()
}

fn agent_body(office: &Office, pattern: &str) -> Value {
    json!({
        "body_version": 1,
        "name_pattern": pattern,
        "command": "claude",
        "working_folder": office.work_path(),
    })
}

fn team_body(office: &Office, member_count: u64) -> Value {
    let entry = |pattern: &str| json!({"name_pattern": pattern, "command": "claude", "working_folder": office.work_path()});
    json!({
        "body_version": 1,
        "lead": entry("{team}-lead"),
        "members": [{"entry": entry("{team}-member-{n}"), "count": member_count}],
    })
}

async fn save(client: &mut Client, name: &str, kind: &str, body: Value) -> String {
    let result = ok(
        client,
        "template.save",
        json!({"name": name, "kind": kind, "body": body}),
    )
    .await;
    result["template"]["id"].as_str().expect("id").to_owned()
}

async fn create_ceo(client: &mut Client, office: &Office) -> String {
    let result = ok(
        client,
        "agent.create",
        json!({
            "name": "Chief",
            "command": "claude",
            "working_folder": office.work_path(),
            "role": {"kind": "ceo"},
        }),
    )
    .await;
    result["id"].as_str().expect("id").to_owned()
}

async fn agent_names(client: &mut Client) -> Vec<String> {
    let agents = ok(client, "agent.list", json!({})).await;
    let mut names: Vec<String> = agents
        .as_array()
        .expect("agents")
        .iter()
        .map(|agent| agent["name"].as_str().expect("name").to_owned())
        .collect();
    names.sort();
    names
}

/// Reads events until one of the type `last` arrives, and returns them all.
async fn events_until(client: &mut Client, last: &str) -> Vec<Value> {
    let mut events = Vec::new();
    loop {
        let message = timeout(EVENT_WAIT, client.wire.receive_json())
            .await
            .expect("an event arrives in time");
        let Some(kind) = message["type"].as_str().map(str::to_owned) else {
            continue;
        };
        events.push(message);
        if kind == last {
            return events;
        }
    }
}

fn count_of(events: &[Value], kind: &str) -> usize {
    events.iter().filter(|event| event["type"] == kind).count()
}

#[tokio::test]
async fn an_agent_template_made_from_an_agent_creates_numbered_agents() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let source = client
        .create_agent(&office, "reviewer", "claude", None)
        .await;
    let copied = ok(
        &mut client,
        "template.from_agent",
        json!({"agent_id": source}),
    )
    .await;
    assert_eq!(copied["body"]["name_pattern"], "reviewer-{n}");
    let template_id = save(&mut client, "Reviewer", "agent", copied["body"].clone()).await;

    let first = ok(
        &mut client,
        "template.create_agent",
        json!({"template_id": template_id}),
    )
    .await;
    let second = ok(
        &mut client,
        "template.create_agent",
        json!({"template_id": template_id}),
    )
    .await;

    assert_eq!(first["agent"]["name"], "reviewer-1");
    assert_eq!(second["agent"]["name"], "reviewer-2");
    assert_eq!(first["agent"]["status"], "stopped");
}

#[tokio::test]
async fn two_teams_from_one_template_get_their_names_from_the_patterns() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = create_ceo(&mut client, &office).await;
    let template_id = save(
        &mut client,
        "Squad",
        "team",
        team_body(&office, MEMBER_COUNT),
    )
    .await;

    let mut team_ids = Vec::new();
    for team_name in ["Alpha", "Beta"] {
        let created = ok(
            &mut client,
            "template.create_team",
            json!({"template_id": template_id, "team_name": team_name, "parent_agent_id": ceo}),
        )
        .await;
        let agents = created["agents"].as_array().expect("agents");
        assert_eq!(
            agents.len(),
            1 + usize::try_from(MEMBER_COUNT).expect("count")
        );
        assert_eq!(agents[0]["name"], format!("{team_name}-lead"));
        assert_eq!(agents[0]["role"], "lead");
        assert_eq!(agents[1]["name"], format!("{team_name}-member-1"));
        assert_eq!(agents[6]["name"], format!("{team_name}-member-6"));
        assert_eq!(agents[6]["role"], "member");
        team_ids.push(created["team"]["id"].as_str().expect("id").to_owned());
    }

    assert_ne!(team_ids[0], team_ids[1]);
    let tree = ok(&mut client, "hierarchy.tree", json!({})).await;
    let teams = tree["teams"].as_array().expect("teams");
    assert_eq!(teams.len(), 2);
    for team in teams {
        assert_eq!(team["members"].as_array().expect("members").len(), 6);
    }
}

#[tokio::test]
async fn a_taken_team_name_creates_nothing() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = create_ceo(&mut client, &office).await;
    let template_id = save(&mut client, "Squad", "team", team_body(&office, 2)).await;
    let request = json!({"template_id": template_id, "team_name": "Alpha", "parent_agent_id": ceo});
    ok(&mut client, "template.create_team", request.clone()).await;
    let before = agent_names(&mut client).await;

    let answer = client.ask("template.create_team", request).await;

    let (code, message) = error_text(&answer);
    assert_eq!(code, "conflict");
    assert_eq!(message, "team name \"Alpha\" is already used");
    assert_eq!(agent_names(&mut client).await, before);
}

#[tokio::test]
async fn export_then_import_into_a_fresh_office_keeps_everything_but_env_values() {
    let first = Office::open().await;
    let mut client = first.client().await;
    let mut body = agent_body(&first, "worker-{n}");
    body["additions"] = json!({
        "flags": ["--fast"],
        "env": [{"name": "API_KEY", "value": SECRET_VALUE}, {"name": "MODE", "value": null}],
        "startup_prompt": "Start with the open change.",
    });
    body["role"] = json!("member");
    body["own_worktree"] = json!(true);
    save(&mut client, "Worker", "agent", body).await;
    let mut team = team_body(&first, 3);
    team["lead"]["additions"] = json!({"env": [{"name": "LEAD_KEY", "value": SECRET_VALUE}]});
    team["members"][0]["entry"]["own_worktree"] = json!(true);
    save(&mut client, "Squad", "team", team).await;
    let exported = ok(&mut client, "template.export", json!({})).await;
    let text = exported["text"].as_str().expect("text").to_owned();
    assert!(!text.contains(SECRET_VALUE), "the file holds an env value");

    let second = Office::open().await;
    let mut fresh = second.client().await;
    let preview = ok(&mut fresh, "template.import.preview", json!({"text": text})).await;
    assert_eq!(preview["templates"].as_array().expect("templates").len(), 2);
    let listed_after_preview = ok(&mut fresh, "template.list", json!({})).await;
    assert_eq!(
        listed_after_preview["templates"],
        json!([]),
        "a preview saves nothing"
    );
    ok(&mut fresh, "template.import.confirm", json!({"text": text})).await;

    let original = ok(&mut client, "template.list", json!({})).await;
    let imported = ok(&mut fresh, "template.list", json!({})).await;
    assert_eq!(
        without_env_values(&original["templates"]),
        without_env_values(&imported["templates"])
    );
    assert!(!imported.to_string().contains(SECRET_VALUE));
    assert_own_worktree_choices(&imported["templates"]);
}

/// The own worktree choice (0010 FR-009) comes back from an import as it was
/// saved: on for the agent template and the members, off for the lead.
fn assert_own_worktree_choices(templates: &Value) {
    let body_of = |name: &str| {
        templates
            .as_array()
            .expect("templates")
            .iter()
            .find(|template| template["name"] == name)
            .expect("template")["body"]
            .clone()
    };
    assert_eq!(body_of("Worker")["own_worktree"], true);
    let squad = body_of("Squad");
    assert_eq!(squad["lead"]["own_worktree"], false);
    assert_eq!(squad["members"][0]["entry"]["own_worktree"], true);
}

/// Each template as `{name, kind, body}` with every env value null, in the
/// list's order, so two offices compare on what an import must keep.
fn without_env_values(templates: &Value) -> Vec<Value> {
    templates
        .as_array()
        .expect("templates")
        .iter()
        .map(|template| {
            let mut body = template["body"].clone();
            null_env_values(&mut body);
            null_env_values(&mut body["lead"]);
            if let Some(members) = body["members"].as_array_mut() {
                for member in members {
                    null_env_values(&mut member["entry"]);
                }
            }
            json!({"name": template["name"], "kind": template["kind"], "body": body})
        })
        .collect()
}

fn null_env_values(agent_body: &mut Value) {
    if let Some(env) = agent_body["additions"]["env"].as_array_mut() {
        for entry in env {
            entry["value"] = Value::Null;
        }
    }
}

#[tokio::test]
async fn an_import_of_a_file_with_an_env_value_saves_no_value() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let text = json!({
        "format": "office-harness-templates",
        "version": 1,
        "templates": [{
            "name": "Crafted",
            "kind": "agent",
            "body": {
                "body_version": 1,
                "name_pattern": "x-{n}",
                "command": "claude",
                "additions": {"env": [{"name": "LD_PRELOAD", "value": "/tmp/x.so"}]},
            },
        }],
    })
    .to_string();

    let preview = ok(
        &mut client,
        "template.import.preview",
        json!({"text": text}),
    )
    .await;
    let confirmed = ok(
        &mut client,
        "template.import.confirm",
        json!({"text": text}),
    )
    .await;

    let env = &preview["templates"][0]["body"]["additions"]["env"];
    assert_eq!(env, &json!([{"name": "LD_PRELOAD", "value": null}]));
    let saved = &confirmed["templates"][0]["body"]["additions"]["env"];
    assert_eq!(saved, &json!([{"name": "LD_PRELOAD", "value": null}]));
}

#[tokio::test]
async fn an_import_text_over_256_kib_is_refused_as_invalid() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let text = " ".repeat(MAX_FILE_BYTES + 1);

    for kind in ["template.import.preview", "template.import.confirm"] {
        let answer = client.ask(kind, json!({"text": text})).await;

        let (code, message) = error_text(&answer);
        assert_eq!(code, "invalid", "{kind}");
        assert_eq!(message, "the file is larger than 256 KiB", "{kind}");
    }
}

#[tokio::test]
async fn editing_and_deleting_a_template_change_no_agent_or_team() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = create_ceo(&mut client, &office).await;
    let template_id = save(&mut client, "Squad", "team", team_body(&office, 2)).await;
    ok(
        &mut client,
        "template.create_team",
        json!({"template_id": template_id, "team_name": "Alpha", "parent_agent_id": ceo}),
    )
    .await;
    let agents_before = ok(&mut client, "agent.list", json!({})).await;
    let tree_before = ok(&mut client, "hierarchy.tree", json!({})).await;

    let mut edited = team_body(&office, 5);
    edited["lead"]["name_pattern"] = json!("changed-lead");
    ok(
        &mut client,
        "template.save",
        json!({"template_id": template_id, "name": "Squad", "kind": "team", "body": edited}),
    )
    .await;
    ok(
        &mut client,
        "template.delete",
        json!({"template_id": template_id}),
    )
    .await;

    assert_eq!(
        ok(&mut client, "agent.list", json!({})).await,
        agents_before
    );
    assert_eq!(
        ok(&mut client, "hierarchy.tree", json!({})).await,
        tree_before
    );
}

#[tokio::test]
async fn another_client_hears_each_created_agent_once_and_nothing_for_a_failed_creation() {
    let office = Office::open().await;
    let mut actor = office.client().await;
    let mut watcher = office.client().await;
    let ceo = create_ceo(&mut actor, &office).await;
    let template_id = save(&mut actor, "Squad", "team", team_body(&office, 2)).await;
    // Everything so far is drained up to this marker.
    events_until(&mut watcher, "template.changed").await;

    ok(
        &mut actor,
        "template.create_team",
        json!({"template_id": template_id, "team_name": "Alpha", "parent_agent_id": ceo}),
    )
    .await;
    let created = events_until(&mut watcher, "hierarchy.changed").await;

    assert_eq!(count_of(&created, "agent.changed"), 3);
    assert_eq!(created.last().expect("event")["cause"], "team.created");

    // A creation that fails (the team name is taken) is followed by a save,
    // whose event closes the window to look at.
    let failed = actor
        .ask(
            "template.create_team",
            json!({"template_id": template_id, "team_name": "Alpha", "parent_agent_id": ceo}),
        )
        .await;
    assert_eq!(failed["ok"], false);
    save(&mut actor, "Marker", "agent", agent_body(&office, "m-{n}")).await;
    let after_failure = events_until(&mut watcher, "template.changed").await;
    assert_eq!(count_of(&after_failure, "agent.changed"), 0);
    assert_eq!(count_of(&after_failure, "hierarchy.changed"), 0);
}

/// Makes every team seat after the first fail to write, as a disk fault in
/// the middle of a creation would: the lead is seated, the first member is not.
fn fail_member_seats(office: &Office) {
    let connection = Connection::open(office.serving.office.config.database_path())
        .expect("the office database");
    connection
        .execute_batch(
            "CREATE TRIGGER inject_member_failure BEFORE INSERT ON hierarchy_team_members \
             WHEN (SELECT COUNT(*) FROM hierarchy_team_members) >= 1 \
             BEGIN SELECT RAISE(ABORT, 'injected member failure'); END;",
        )
        .expect("the trigger");
}

#[tokio::test]
async fn a_creation_that_fails_after_a_write_sends_only_the_records_of_its_entries() {
    let office = Office::open().await;
    let mut actor = office.client().await;
    let mut watcher = office.client().await;
    let ceo = create_ceo(&mut actor, &office).await;
    let template_id = save(&mut actor, "Squad", "team", team_body(&office, 2)).await;
    events_until(&mut watcher, "template.changed").await;
    fail_member_seats(&office);

    let failed = actor
        .ask(
            "template.create_team",
            json!({"template_id": template_id, "team_name": "Alpha", "parent_agent_id": ceo}),
        )
        .await;
    assert_eq!(failed["ok"], false);
    assert!(error_text(&failed).1.contains("injected member failure"));
    save(&mut actor, "Marker", "agent", agent_body(&office, "m-{n}")).await;
    let heard = events_until(&mut watcher, "template.changed").await;

    // The lead and the first member were created, then both were removed:
    // a Create and a Delete entry for each, newest removal first.
    let records: Vec<&str> = heard
        .iter()
        .filter(|event| event["type"] == "action.recorded")
        .map(|event| event["entry"]["action"].as_str().expect("action"))
        .collect();
    assert_eq!(records, ["create", "create", "delete", "delete"]);
    assert_eq!(count_of(&heard, "agent.changed"), 0);
    assert_eq!(count_of(&heard, "hierarchy.changed"), 0);
    assert_eq!(heard.len(), 5, "four records and the marker: {heard:?}");
    assert_eq!(agent_names(&mut actor).await, ["Chief"]);
}

#[tokio::test]
async fn a_save_and_a_delete_each_send_the_full_list_to_other_clients() {
    let office = Office::open().await;
    let mut actor = office.client().await;
    let mut watcher = office.client().await;

    let template_id = save(
        &mut actor,
        "Reviewer",
        "agent",
        agent_body(&office, "r-{n}"),
    )
    .await;
    let saved = events_until(&mut watcher, "template.changed").await;
    ok(
        &mut actor,
        "template.delete",
        json!({"template_id": template_id}),
    )
    .await;
    let deleted = events_until(&mut watcher, "template.changed").await;

    let names = |events: &[Value]| {
        events.last().expect("event")["templates"]
            .as_array()
            .expect("templates")
            .iter()
            .map(|template| template["name"].as_str().expect("name").to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&saved), vec!["Reviewer"]);
    assert_eq!(names(&deleted), Vec::<String>::new());
}

#[tokio::test]
async fn a_field_a_request_does_not_take_is_refused_by_name() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let template_id = save(
        &mut client,
        "Reviewer",
        "agent",
        agent_body(&office, "r-{n}"),
    )
    .await;

    let answer = client
        .ask(
            "template.create_agent",
            json!({"template_id": template_id, "templateid": template_id}),
        )
        .await;

    let (code, message) = error_text(&answer);
    assert_eq!(code, "invalid");
    assert_eq!(
        message,
        "templateid is not a field of template.create_agent"
    );
    assert_eq!(agent_names(&mut client).await, Vec::<String>::new());
}

#[tokio::test]
async fn a_kind_that_does_not_match_the_body_is_refused() {
    let office = Office::open().await;
    let mut client = office.client().await;

    let answer = client
        .ask(
            "template.save",
            json!({"name": "Odd", "kind": "team", "body": agent_body(&office, "r-{n}")}),
        )
        .await;

    let (code, message) = error_text(&answer);
    assert_eq!(code, "invalid");
    assert_eq!(message, "kind is \"team\" but the body is an agent body");
}

#[tokio::test]
async fn a_template_name_that_is_taken_is_a_conflict_and_an_unknown_id_is_not_found() {
    let office = Office::open().await;
    let mut client = office.client().await;
    save(
        &mut client,
        "Reviewer",
        "agent",
        agent_body(&office, "r-{n}"),
    )
    .await;

    let taken = client
        .ask(
            "template.save",
            json!({"name": "reviewer", "kind": "agent", "body": agent_body(&office, "r-{n}")}),
        )
        .await;
    let missing = client
        .ask(
            "template.get",
            json!({"template_id": "00000000-0000-4000-8000-000000000000"}),
        )
        .await;

    assert_eq!(error_text(&taken).0, "conflict");
    assert_eq!(error_text(&missing).0, "not_found");
}

/// A1 and PB-3: agents made from templates are created, not started, and
/// no template step writes a CLI's configuration, in the home folders the
/// CLIs read or in the project folder.
#[tokio::test]
async fn template_work_starts_no_agent_and_writes_no_cli_configuration() {
    let office = Office::open().await;
    let fake_folders = office.clis.tool_folder().parent().expect("fake folders");
    let watched = [fake_folders, office.work_folder.path()];
    let before = entries_under(&watched);
    let mut client = office.client().await;
    let ceo = create_ceo(&mut client, &office).await;
    let mut body = agent_body(&office, "worker-{n}");
    body["additions"] = json!({
        "flags": ["--fast"],
        "env": [{"name": "API_KEY", "value": SECRET_VALUE}],
        "startup_prompt": "Start with the open change.",
    });
    let agent_template = save(&mut client, "Worker", "agent", body).await;
    let team_template = save(&mut client, "Squad", "team", team_body(&office, 2)).await;

    ok(
        &mut client,
        "template.create_agent",
        json!({"template_id": agent_template}),
    )
    .await;
    ok(
        &mut client,
        "template.create_team",
        json!({"template_id": team_template, "team_name": "Alpha", "parent_agent_id": ceo}),
    )
    .await;
    ok(&mut client, "template.export", json!({})).await;

    let agents = ok(&mut client, "agent.list", json!({})).await;
    let agents = agents.as_array().expect("agents");
    assert_eq!(agents.len(), 5, "the CEO, one worker, and a team of three");
    for agent in agents {
        assert_eq!(agent["status"], "stopped", "{agent}");
    }
    assert_eq!(entries_under(&watched), before);
}

fn action_record_count(office: &Office) -> i64 {
    Connection::open(office.serving.office.config.database_path())
        .expect("the office database")
        .query_row("SELECT COUNT(*) FROM audit_actions", [], |row| row.get(0))
        .expect("the action record")
}

/// A10: saving, editing, exporting, importing, and deleting templates write
/// no action record; an agent created from one does (PB-2), which shows the
/// count can move.
#[tokio::test]
async fn template_changes_write_no_action_record() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let before = action_record_count(&office);

    let template_id = save(&mut client, "Worker", "agent", agent_body(&office, "w-{n}")).await;
    ok(
        &mut client,
        "template.save",
        json!({"template_id": template_id, "name": "Worker", "kind": "agent",
               "body": agent_body(&office, "changed-{n}")}),
    )
    .await;
    let exported = ok(&mut client, "template.export", json!({})).await;
    ok(
        &mut client,
        "template.delete",
        json!({"template_id": template_id}),
    )
    .await;
    let text = json!({"text": exported["text"]});
    ok(&mut client, "template.import.preview", text.clone()).await;
    ok(&mut client, "template.import.confirm", text).await;
    assert_eq!(action_record_count(&office), before);

    let listed = ok(&mut client, "template.list", json!({})).await;
    let imported = listed["templates"][0]["id"].clone();
    ok(
        &mut client,
        "template.create_agent",
        json!({"template_id": imported}),
    )
    .await;
    assert_eq!(action_record_count(&office), before + 1);
}
