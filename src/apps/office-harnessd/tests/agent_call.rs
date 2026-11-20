// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! An agent calls the office from inside its session (0002, T4). The fake CLI
//! runs the real `office-harnessd agent-call` helper with the environment
//! that the server gave the session.

mod support;

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;

use agent_infrastructure::persistence::SharedConnection;
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;
use serde_json::{Value, json};
use support::office_client::{Client, Office, wait_for};

/// The real server binary that the fake CLI runs as the helper.
const HELPER: &str = env!("CARGO_BIN_EXE_office-harnessd");

/// A fake CLI that records what the helper said for one call, then waits.
/// `calls` is a list of `<name> <request type>` pairs; the helper's answer
/// goes to `<records>/<name>.out`, and its environment to `<records>/env`.
fn call_script(records: &Path, calls: &[(&str, &str)]) -> String {
    let folder = records.display();
    let mut body = format!("env > {folder}/env.part\nmv {folder}/env.part {folder}/env\n");
    for (name, kind) in calls {
        let _ = writeln!(
            body,
            "{HELPER} agent-call {kind} > {folder}/{name}.out 2> {folder}/{name}.err\nprintf '%s' $? > {folder}/{name}.code"
        );
    }
    body.push_str("sleep 60\n");
    body
}

async fn read_output(records: &Path, name: &str) -> (String, String) {
    // A redirect creates its file empty and fills it afterwards, so an
    // existing file is not a whole one: the wait needs content, not
    // existence.
    let code = wait_for("the helper's exit code", || {
        fs::read_to_string(records.join(format!("{name}.code")))
            .ok()
            .filter(|text| !text.is_empty())
    })
    .await;
    let out = wait_for("the helper's answer", || {
        fs::read_to_string(records.join(format!("{name}.out")))
            .ok()
            .filter(|text| !text.is_empty())
    })
    .await;
    (code, out)
}

fn environment_value(records: &Path, name: &str) -> String {
    let env = fs::read_to_string(records.join("env")).expect("env");
    env.lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=")))
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test]
async fn an_agent_proves_itself_and_the_office_names_it() {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &call_script(office.records_path(), &[("whoami", "agent.whoami")]),
    );
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;

    let (code, out) = read_output(office.records_path(), "whoami").await;

    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], true);
    assert_eq!(answer["result"]["agent_id"], agent);
}

#[tokio::test]
async fn an_agent_may_not_ask_for_a_type_outside_the_allowlist() {
    let office = Office::open().await;
    let kinds = [
        ("terminal", "terminal.input"),
        ("delete", "agent.delete"),
        ("team", "team.create"),
        ("tree", "hierarchy.tree"),
        ("restart", "server.restart"),
        ("settings", "settings.set"),
        ("needs_you_list", "needs_you.list"),
        ("needs_you_dismiss", "needs_you.dismiss"),
    ];
    office
        .clis
        .add("claude", &call_script(office.records_path(), &kinds));
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;

    for (name, kind) in kinds {
        let (code, out) = read_output(office.records_path(), name).await;
        let answer: Value = serde_json::from_str(out.trim()).expect("json");
        assert_eq!(code, "1", "{kind}: {out}");
        assert_eq!(answer["ok"], false, "{kind}");
        assert_eq!(answer["error"]["code"], "refused", "{kind}");
    }
}

#[tokio::test]
async fn the_child_gets_its_own_credential_and_never_the_office_token() {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &call_script(office.records_path(), &[("whoami", "agent.whoami")]),
    );
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;
    read_output(office.records_path(), "whoami").await;

    let credential = environment_value(office.records_path(), "OFFICE_HARNESS_AGENT_TOKEN");
    let office_token = office.serving.office.token();
    let env_text = fs::read_to_string(office.records_path().join("env")).expect("env");

    assert!(credential.starts_with(&format!("{agent}:")), "{credential}");
    assert!(!env_text.contains(&office_token), "the office token leaked");
    for name in ["OFFICE_HARNESS_CLI", "OFFICE_HARNESS_SERVER_FILE"] {
        assert_ne!(environment_value(office.records_path(), name), "", "{name}");
    }
}

/// Runs the helper with the given credential. The server of this test runs
/// in this test's runtime, so the helper runs on a blocking thread: waiting
/// for it on the runtime thread would stop the server it talks to.
async fn call_with(credential: &str, server_file: &str) -> Output {
    let credential = credential.to_owned();
    let server_file = server_file.to_owned();
    tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", "agent.whoami"])
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task")
}

#[tokio::test]
async fn a_stopped_agents_credential_stops_working_and_a_restart_gets_a_new_one() {
    let office = Office::open().await;
    office.clis.add(
        "claude",
        &call_script(office.records_path(), &[("whoami", "agent.whoami")]),
    );
    let mut client = office.client().await;
    let agent = client.create_agent(&office, "Ana", "claude", None).await;
    client.start(&agent).await;
    read_output(office.records_path(), "whoami").await;
    let first = environment_value(office.records_path(), "OFFICE_HARNESS_AGENT_TOKEN");
    let server_file = environment_value(office.records_path(), "OFFICE_HARNESS_SERVER_FILE");
    // The credential works while the session runs.
    let live = call_with(&first, &server_file).await;
    assert!(live.status.success(), "{live:?}");

    let stopped = client.ask("agent.stop", json!({"agent_id": agent})).await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    client.wait_for_status(&agent, "stopped").await;

    let refused = call_with(&first, &server_file).await;
    assert!(!refused.status.success());
    let secret = first.split_once(':').expect("shape").1;
    let shown = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(!shown.contains(secret), "the secret was printed");

    // A new session of the same agent gets a different credential, and the
    // old one stays dead.
    fs::remove_file(office.records_path().join("env")).expect("old env");
    client.start(&agent).await;
    wait_for("the new session's environment", || {
        fs::read_to_string(office.records_path().join("env")).ok()
    })
    .await;
    let second = environment_value(office.records_path(), "OFFICE_HARNESS_AGENT_TOKEN");
    assert_ne!(first, second);
    assert!(!call_with(&first, &server_file).await.status.success());
    assert!(call_with(&second, &server_file).await.status.success());
}

/// A fake CLI that dumps its environment, calls the helper once with the
/// given type and request, keeps both, and stays alive for the test (the
/// pattern of `agent_directive.rs`).
fn calling_script(records: &Path, name: &str, kind: &str, request: &str) -> String {
    let folder = records.display();
    format!(
        "env > {folder}/env.part\nmv {folder}/env.part {folder}/env\n\
         {HELPER} agent-call {kind} '{request}' > {folder}/{name}.out 2> {folder}/{name}.err\n\
         printf '%s' $? > {folder}/{name}.code\nsleep 30\n"
    )
}

/// The action entries the wire holds for `agent_id` (as in
/// `agent_directive.rs`).
async fn actions_of(client: &mut Client, agent_id: &str) -> Vec<Value> {
    let actions = client
        .ask("agent.actions", json!({"agent_id": agent_id}))
        .await;
    assert_eq!(actions["ok"], true, "{actions}");
    actions["result"].as_array().expect("entries").clone()
}

/// Gives an agent its team through the same store the server built (as
/// `agent_directive.rs` writes a place): the wire has no team.create with a
/// picked lead yet in these tests, so the test writes the team directly.
/// The lead's place columns go with the team row, and the members an agent
/// creates then show under it in `hierarchy.tree`.
fn place_lead_with_team(office: &Office, lead_agent_id: &str, team_name: &str) {
    let connection =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
    let store: Arc<dyn HierarchyStore> = Arc::new(HierarchyRepository::new(
        SharedConnection::new(connection).handle(),
        AgentPlaces::new(),
    ));
    hierarchy_application::CreateTeamHandler::new(store)
        .handle(
            &hierarchy_application::commands::create_team::CreateTeamRequest {
                name: team_name.to_owned(),
                project_id: None,
                lead_agent_id: lead_agent_id.parse().expect("agent id"),
            },
        )
        .expect("team");
}

/// A lead agent-creates a member over agent-call: the child is placed in
/// the tree under the lead, in the lead's own place (FR-008, SC-006).
#[tokio::test]
async fn a_lead_creates_a_member_through_agent_call_and_the_tree_shows_it() {
    let office = Office::open().await;
    // The lead's CLI creates a member through the helper, then keeps the
    // session alive.
    // The lead holds a team, as the user sets it up: a member can only
    // sit under a lead, so the caller itself must be one.
    office.clis.add("member-cli", "echo ready; sleep 30");
    let request = json!({
        "name": "Bo",
        "command": "member-cli",
        "working_folder": office.work_path(),
    });
    office.clis.add(
        "lead-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &request.to_string(),
        ),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    // The lead holds a team, as the user sets it up: a member can only
    // sit under a lead, so the caller itself must be one.
    place_lead_with_team(&office, &lead, "The Team");
    client.start(&lead).await;

    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], true, "{answer}");
    let member = answer["result"]["id"].as_str().expect("id").to_owned();

    // The tree names the new agent with its role and parent.
    let tree = client.ask("hierarchy.tree", json!({})).await;
    assert_eq!(tree["ok"], true, "{tree}");
    // The lead holds a team; the member sits under it with the member role.
    let teams = tree["result"]["teams"].as_array().expect("teams");
    assert_eq!(teams.len(), 1, "{tree}");
    assert_eq!(teams[0]["lead"]["id"], json!(lead), "{tree}");
    let members = teams[0]["members"].as_array().expect("members");
    assert_eq!(members.len(), 1, "{tree}");
    assert_eq!(members[0]["id"], json!(member), "{tree}");
    assert_eq!(members[0]["role"], "member", "{tree}");

    // The member record reads back like any `agent.create` answer.
    let listed = client.ask("agent.list", json!({})).await;
    let found = listed["result"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| agent["id"] == json!(member))
        .expect("the member is listed")
        .clone();
    assert_eq!(found["name"], "Bo", "{found}");

    office.serving.stop().await;
}

/// A lead stops its member over agent-call (FR-008).
#[tokio::test]
async fn a_lead_stops_its_member_through_agent_call() {
    let office = Office::open().await;
    office.clis.add("member-cli", "echo ready; sleep 30");
    let request = json!({
        "name": "Bo",
        "command": "member-cli",
        "working_folder": office.work_path(),
    });
    office.clis.add(
        "lead-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &request.to_string(),
        ),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place_lead_with_team(&office, &lead, "The Team");
    client.start(&lead).await;
    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    let member = answer["result"]["id"].as_str().expect("id").to_owned();
    client.start(&member).await;
    client.wait_for_status(&member, "working").await;

    // A second helper call from the same live session stops the member:
    // the test holds the session credential the server gave the lead.
    let stop = json!({"agent_id": member});
    let env = fs::read_to_string(office.records_path().join("env")).expect("env");
    let value = |name: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .expect(name)
            .to_owned()
    };
    let credential = value("OFFICE_HARNESS_AGENT_TOKEN");
    let server_file = value("OFFICE_HARNESS_SERVER_FILE");
    let outcome = {
        let credential = credential.clone();
        let server_file = server_file.clone();
        let stop = stop.clone();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new(HELPER)
                .args(["agent-call", "agent.stop", &stop.to_string()])
                .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
                .env("OFFICE_HARNESS_SERVER_FILE", server_file)
                .output()
                .expect("helper")
        })
        .await
        .expect("helper task")
    };
    assert!(outcome.status.success(), "{outcome:?}");
    client.wait_for_status(&member, "stopped").await;

    office.serving.stop().await;
}

/// An unsupported command (here the project request the allowlist already
/// names) is refused and recorded; so is an `additions` field (0002
/// contract, agent caller).
#[tokio::test]
async fn an_unsupported_command_or_field_is_refused_and_recorded() {
    let office = Office::open().await;
    let create = json!({
        "name": "Bo",
        "command": "echo ready; sleep 30",
        "working_folder": office.work_path(),
    });
    office.clis.add(
        "lead-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &create.to_string(),
        ),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place_lead_with_team(&office, &lead, "The Team");
    client.start(&lead).await;
    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    let member = answer["result"]["id"].as_str().expect("id").to_owned();

    let env = fs::read_to_string(office.records_path().join("env")).expect("env");
    let value = |name: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .expect(name)
            .to_owned()
    };
    let credential = value("OFFICE_HARNESS_AGENT_TOKEN");
    let server_file = value("OFFICE_HARNESS_SERVER_FILE");

    // The additions field is refused before any use case runs.
    let additions = json!({
        "name": "Cy",
        "command": "echo hi",
        "additions": {"flags": ["--x"]},
    });
    let with_additions = {
        let credential = credential.clone();
        let server_file = server_file.clone();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new(HELPER)
                .args(["agent-call", "agent.create", &additions.to_string()])
                .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
                .env("OFFICE_HARNESS_SERVER_FILE", server_file)
                .output()
                .expect("helper")
        })
        .await
        .expect("helper task")
    };
    assert!(!with_additions.status.success(), "{with_additions:?}");
    let refused: Value =
        serde_json::from_str(String::from_utf8_lossy(&with_additions.stdout).trim()).expect("json");
    assert_eq!(refused["error"]["code"], "invalid", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .expect("message")
            .contains("additions"),
        "{refused}"
    );

    // `agent.delete` is outside the allowlist: refused and recorded.
    let delete = json!({"agent_id": member});
    let delete_call = {
        let credential = credential.clone();
        let server_file = server_file.clone();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new(HELPER)
                .args(["agent-call", "agent.delete", &delete.to_string()])
                .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
                .env("OFFICE_HARNESS_SERVER_FILE", server_file)
                .output()
                .expect("helper")
        })
        .await
        .expect("helper task")
    };
    assert!(!delete_call.status.success(), "{delete_call:?}");
    let refused: Value =
        serde_json::from_str(String::from_utf8_lossy(&delete_call.stdout).trim()).expect("json");
    assert_eq!(refused["error"]["code"], "refused", "{refused}");

    // Both refusals sit in the action record with the lead as actor
    // (FR-008, FR-009).
    let entries = actions_of(&mut client, &lead).await;
    let recorded = entries
        .iter()
        .filter(|entry| entry["action"] == "refused")
        .count();
    assert!(recorded >= 2, "{entries:?}");

    office.serving.stop().await;
}

/// Places one agent through the same store the server built, as
/// `agent_subtree.rs` does for its tree: the caller must hold its place
/// before its create can raise a child (A4).
fn place(office: &Office, agent_id: &str, role: hierarchy_domain::Role, parent: Option<&str>) {
    let connection =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
    let store: Arc<dyn HierarchyStore> = Arc::new(HierarchyRepository::new(
        SharedConnection::new(connection).handle(),
        AgentPlaces::new(),
    ));
    hierarchy_application::PlaceAgentHandler::new(store)
        .handle(
            hierarchy_application::commands::place_agent::PlaceAgentRequest {
                agent_id: agent_id.parse().expect("agent id"),
                role,
                parent_agent_id: parent.map(|id| id.parse().expect("agent id")),
                team_id: None,
            },
        )
        .expect("placement");
}

/// A CEO agent raises a team lead over agent-call, naming the team the lead
/// will hold: the lead becomes the leadless team's lead, in the row and in
/// the tree (A4, FR-003).
#[tokio::test]
async fn a_ceo_creates_a_lead_through_agent_call_and_the_row_carries_the_team() {
    let office = Office::open().await;
    office.clis.add("lead-cli", "echo ready; sleep 30");
    let mut client = office.client().await;
    let ceo = client.create_agent(&office, "Chief", "ceo-cli", None).await;
    place(&office, &ceo, hierarchy_domain::Role::Ceo, None);
    // The team exists first, without a lead of its own in the tree: the
    // user's `team.create` needs a lead agent, so this test deletes the
    // placeholder again, which leaves the team row but clears its seat.
    // The CEO's create then names that team for the lead it raises. The
    // command names the script the test writes later, with the team id
    // inside: a script only has to exist when the session starts.
    let scout = client
        .create_agent(&office, "Scout", "member-cli", None)
        .await;
    let team = client
        .ask(
            "team.create",
            json!({"name": "The Team", "lead_agent_id": scout}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");
    let team_id = team["result"]["id"].as_str().expect("team id").to_owned();
    let deleted = client.ask("agent.delete", json!({"agent_id": scout})).await;
    assert_eq!(deleted["ok"], true, "{deleted}");
    let lead_request = json!({
        "name": "Bo",
        "command": "lead-cli",
        "working_folder": office.work_path(),
        "role": {"kind": "lead", "team_id": team_id},
    });
    office.clis.add(
        "ceo-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &lead_request.to_string(),
        ),
    );
    client.start(&ceo).await;

    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], true, "{answer}");
    let lead = answer["result"]["id"].as_str().expect("id").to_owned();

    // The created row is placed with the team the CEO named: the role
    // column says lead, the parent is the CEO, and the team column carries
    // the team (the gateway must not drop it, PR 40 r2 LOW).
    let connection =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
    let (role, parent, row_team): (String, Option<String>, Option<String>) = connection
        .query_row(
            "SELECT role, parent_agent_id, team_id FROM agent_agents WHERE id = ?1",
            [&lead],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("the placed row");
    assert_eq!(role, "lead", "{parent:?}");
    assert_eq!(parent, Some(ceo.clone()), "{role}");
    assert_eq!(row_team, Some(team_id.clone()), "{role}");

    // The placement set the team's lead column in the same write, so the
    // tree shows the new lead on the team at once (the placement did not
    // only fill the agent row's place columns).
    let tree = client.ask("hierarchy.tree", json!({})).await;
    assert_eq!(tree["ok"], true, "{tree}");
    let teams = tree["result"]["teams"].as_array().expect("teams");
    assert_eq!(teams.len(), 1, "{tree}");
    assert_eq!(teams[0]["id"], json!(team_id), "{tree}");
    assert_eq!(teams[0]["lead"]["id"], json!(lead), "{tree}");
    assert_eq!(teams[0]["lead"]["role"], "lead", "{tree}");
    assert_eq!(teams[0]["members"].as_array().expect("members").len(), 0);

    office.serving.stop().await;
}

/// A second lead for a held team is refused over agent-call as `conflict`
/// and recorded: one lead per team (FR-003), and the tree keeps exactly the
/// seated lead.
#[tokio::test]
async fn a_second_lead_for_a_held_team_is_refused_and_recorded() {
    let office = Office::open().await;
    office.clis.add("member-cli", "echo ready; sleep 30");
    let mut client = office.client().await;
    let ceo = client.create_agent(&office, "Chief", "ceo-cli", None).await;
    place(&office, &ceo, hierarchy_domain::Role::Ceo, None);
    let lead = client.create_agent(&office, "Bo", "member-cli", None).await;
    place(&office, &lead, hierarchy_domain::Role::Lead, None);
    let team = client
        .ask(
            "team.create",
            json!({"name": "The Team", "lead_agent_id": lead}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");
    let team_id = team["result"]["id"].as_str().expect("team id").to_owned();
    let second = json!({
        "name": "Cy",
        "command": "member-cli",
        "working_folder": office.work_path(),
        "role": {"kind": "lead", "team_id": team_id},
    });
    office.clis.add(
        "ceo-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &second.to_string(),
        ),
    );
    client.start(&ceo).await;

    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "1", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], false, "{answer}");
    assert_eq!(answer["error"]["code"], "conflict", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("the team already has a lead"),
        "{answer}"
    );

    // The refusal is recorded with the caller as the actor (FR-009).
    let entries = actions_of(&mut client, &ceo).await;
    let refused = entries
        .iter()
        .find(|entry| entry["action"] == "refused")
        .expect("the refusal is recorded");
    assert_eq!(refused["actor"], json!(format!("agent:{ceo}")), "{refused}");
    assert!(
        refused["result"]
            .as_str()
            .expect("result")
            .contains("the team already has a lead"),
        "{refused}"
    );

    // The tree keeps exactly the seated lead: the refused create left no
    // second lead row on the team.
    let tree = client.ask("hierarchy.tree", json!({})).await;
    assert_eq!(tree["ok"], true, "{tree}");
    let teams = tree["result"]["teams"].as_array().expect("teams");
    assert_eq!(teams.len(), 1, "{tree}");
    assert_eq!(teams[0]["id"], json!(team_id), "{tree}");
    assert_eq!(teams[0]["lead"]["id"], json!(lead), "{tree}");
    assert_eq!(teams[0]["members"].as_array().expect("members").len(), 0);

    office.serving.stop().await;
}

/// A CEO agent asking for a lead without its team is refused as `invalid`
/// and recorded: the contract requires a lead's team, and the gateway must
/// not answer success for a lead the tree cannot show (PR 40 r2 LOW).
#[tokio::test]
async fn a_ceo_creating_a_lead_without_a_team_is_refused_and_recorded() {
    let office = Office::open().await;
    let lead_request = json!({
        "name": "Bo",
        "command": "member-cli",
        "working_folder": office.work_path(),
        "role": {"kind": "lead"},
    });
    office.clis.add("member-cli", "echo ready; sleep 30");
    office.clis.add(
        "ceo-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &lead_request.to_string(),
        ),
    );
    let mut client = office.client().await;
    let ceo = client.create_agent(&office, "Chief", "ceo-cli", None).await;
    place(&office, &ceo, hierarchy_domain::Role::Ceo, None);
    client.start(&ceo).await;

    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "1", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["error"]["code"], "invalid", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("role.team_id"),
        "{answer}"
    );

    let entries = actions_of(&mut client, &ceo).await;
    let refused = entries
        .iter()
        .find(|entry| entry["action"] == "refused")
        .expect("the refusal is recorded");
    assert!(
        refused["result"]
            .as_str()
            .expect("result")
            .contains("role.team_id"),
        "{refused}"
    );

    office.serving.stop().await;
}

/// A member caller asking for a child role is refused with the reason its
/// own place gives, and the refusal is recorded with the member as the
/// actor (A4, FR-009).
#[tokio::test]
async fn a_member_caller_asking_for_a_child_role_is_refused_and_recorded() {
    let office = Office::open().await;
    office.clis.add("member-cli", "echo ready; sleep 30");
    let create = json!({
        "name": "Cy",
        "command": "member-cli",
        "working_folder": office.work_path(),
        "role": {"kind": "member"},
    });
    office.clis.add(
        "member-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &create.to_string(),
        ),
    );
    let mut client = office.client().await;
    let lead = client
        .create_agent(&office, "Lead", "member-cli", None)
        .await;
    place(&office, &lead, hierarchy_domain::Role::Lead, None);
    let team = client
        .ask(
            "team.create",
            json!({"name": "The Team", "lead_agent_id": lead}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");
    let member = client
        .create_agent(&office, "Mel", "member-cli", None)
        .await;
    place(
        &office,
        &member,
        hierarchy_domain::Role::Member,
        Some(&lead),
    );
    client.start(&member).await;

    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "1", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], false, "{answer}");
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    let message = answer["error"]["message"].as_str().expect("message");
    assert!(
        message.contains("a member may not create agents"),
        "{answer}"
    );

    // Nothing was created: the member's own create is the only new agent.
    let listed = client.ask("agent.list", json!({})).await;
    let names: Vec<&str> = listed["result"]
        .as_array()
        .expect("agents")
        .iter()
        .filter_map(|agent| agent["name"].as_str())
        .collect();
    assert!(!names.contains(&"Cy"), "{names:?}");

    // The refusal sits in the action record with the member as the actor
    // (FR-009).
    let entries = actions_of(&mut client, &member).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "refused")
        .expect("the refusal is recorded");
    assert_eq!(entry["actor"], json!(format!("agent:{member}")), "{entry}");
    assert!(
        entry["result"]
            .as_str()
            .expect("result")
            .contains("a member may not create agents"),
        "{entry}"
    );

    office.serving.stop().await;
}

/// A lead caller asking for a lead child is refused: its place allows
/// members only (A4, FR-009). The refusal is recorded too.
#[tokio::test]
async fn a_lead_caller_asking_for_a_lead_child_is_refused() {
    let office = Office::open().await;
    let create = json!({
        "name": "Cy",
        "command": "member-cli",
        "working_folder": office.work_path(),
        "role": {"kind": "lead"},
    });
    office.clis.add(
        "lead-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &create.to_string(),
        ),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place(&office, &lead, hierarchy_domain::Role::Lead, None);
    let team = client
        .ask(
            "team.create",
            json!({"name": "The Team", "lead_agent_id": lead}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");
    client.start(&lead).await;

    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "1", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("member"),
        "{answer}"
    );

    let entries = actions_of(&mut client, &lead).await;
    assert!(
        entries.iter().any(|entry| entry["action"] == "refused"),
        "{entries:?}"
    );

    office.serving.stop().await;
}

/// An agent asking to create a ceo is refused whatever its own place (A4,
/// A5): the office has one ceo, and only the user creates it.
#[tokio::test]
async fn an_agent_asking_for_a_ceo_child_is_refused() {
    let office = Office::open().await;
    let create = json!({
        "name": "Cy",
        "command": "member-cli",
        "working_folder": office.work_path(),
        "role": {"kind": "ceo"},
    });
    office.clis.add(
        "lead-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &create.to_string(),
        ),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place(&office, &lead, hierarchy_domain::Role::Lead, None);
    let team = client
        .ask(
            "team.create",
            json!({"name": "The Team", "lead_agent_id": lead}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");
    client.start(&lead).await;

    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "1", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("ceo"),
        "{answer}"
    );
    let listed = client.ask("agent.list", json!({})).await;
    let names: Vec<&str> = listed["result"]
        .as_array()
        .expect("agents")
        .iter()
        .filter_map(|agent| agent["name"].as_str())
        .collect();
    assert!(!names.contains(&"Cy"), "{names:?}");

    office.serving.stop().await;
}

/// A CEO caller may also fill a team seat: a member named under one of its
/// own descendant leads lands under that lead, not under the CEO (A4). The
/// lead's team takes the member (A6).
#[tokio::test]
async fn a_ceo_creates_a_member_under_its_descendant_lead() {
    let office = Office::open().await;
    office.clis.add("member-cli", "echo ready; sleep 30");
    let mut client = office.client().await;
    // The CEO's command names the script the test writes later, with the
    // lead's id inside: a script only has to exist when the session starts.
    let ceo = client.create_agent(&office, "Chief", "ceo-cli", None).await;
    place(&office, &ceo, hierarchy_domain::Role::Ceo, None);
    let lead = client
        .create_agent(&office, "Lead", "member-cli", None)
        .await;
    place(&office, &lead, hierarchy_domain::Role::Lead, None);
    let team = client
        .ask(
            "team.create",
            json!({"name": "The Team", "lead_agent_id": lead}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");

    let create = json!({
        "name": "Cy",
        "command": "member-cli",
        "working_folder": office.work_path(),
        "role": {"kind": "member", "parent_agent_id": lead},
    });
    office.clis.add(
        "ceo-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &create.to_string(),
        ),
    );
    client.start(&ceo).await;

    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], true, "{answer}");
    let member = answer["result"]["id"].as_str().expect("id").to_owned();

    // The member sits in the lead's team, under the lead the CEO named
    // (A6), not under the CEO itself.
    let tree = client.ask("hierarchy.tree", json!({})).await;
    assert_eq!(tree["ok"], true, "{tree}");
    assert_eq!(tree["result"]["ceo"]["id"], json!(ceo), "{tree}");
    let teams = tree["result"]["teams"].as_array().expect("teams");
    assert_eq!(teams.len(), 1, "{tree}");
    assert_eq!(teams[0]["lead"]["id"], json!(lead), "{tree}");
    let members = teams[0]["members"].as_array().expect("members");
    assert_eq!(members.len(), 1, "{tree}");
    assert_eq!(members[0]["id"], json!(member), "{tree}");
    assert_eq!(members[0]["role"], "member", "{tree}");

    office.serving.stop().await;
}

/// The credential and server file the server gave the live session, from
/// the recorded environment.
async fn session_environment(records: &Path) -> (String, String) {
    let env = wait_for("the session's recorded environment", || {
        fs::read_to_string(records.join("env")).ok()
    })
    .await;
    let value = |name: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .expect(name)
            .to_owned()
    };
    (
        value("OFFICE_HARNESS_AGENT_TOKEN"),
        value("OFFICE_HARNESS_SERVER_FILE"),
    )
}

/// Runs the helper with a live session's own credential (as the fake CLI
/// would), so a test can make a second call from the same session.
async fn call_as_session(credential: &str, server_file: &str, kind: &str, request: &str) -> Output {
    let arguments = [kind.to_owned(), request.to_owned()];
    let credential = credential.to_owned();
    let server_file = server_file.to_owned();
    tokio::task::spawn_blocking(move || {
        Command::new(HELPER)
            .args(["agent-call", &arguments[0], &arguments[1]])
            .env("OFFICE_HARNESS_AGENT_TOKEN", credential)
            .env("OFFICE_HARNESS_SERVER_FILE", server_file)
            .output()
            .expect("helper")
    })
    .await
    .expect("helper task")
}

/// A fake CLI that calls the helper on the agent's own id: the credential
/// the server gave the session names the caller, so the request targets
/// itself. The answer goes to `<records>/<name>.out` like `calling_script`.
fn call_on_self_script(records: &Path, name: &str, kind: &str) -> String {
    let folder = records.display();
    format!(
        "env > {folder}/env.part\nmv {folder}/env.part {folder}/env\n\
         agent_id=${{OFFICE_HARNESS_AGENT_TOKEN%%:*}}\n\
         {HELPER} agent-call {kind} \"{{\\\"agent_id\\\":\\\"$agent_id\\\"}}\" > {folder}/{name}.out 2> {folder}/{name}.err\n\
         printf '%s' $? > {folder}/{name}.code\nsleep 30\n"
    )
}

/// The self-target refusals start, `new_task`, and stop stay descendants
/// only (FR-008, FR-009; PR 40 MEDIUM-2). Each test runs one kind: a lead
/// asks its own id over agent-call, the gateway refuses before any use case
/// runs, and the refusal is recorded with the lead as actor.
async fn assert_an_agent_acting_on_itself_is_refused_and_recorded(kind: &str, name: &str) {
    let office = Office::open().await;
    office.clis.add(
        "lead-cli",
        &call_on_self_script(office.records_path(), name, kind),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place_lead_with_team(&office, &lead, "The Team");
    client.start(&lead).await;

    let (code, out) = read_output(office.records_path(), name).await;
    assert_eq!(code, "1", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    assert_eq!(answer["ok"], false, "{answer}");
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("itself"),
        "{answer}"
    );

    // The refusal sits in the action record with the lead as actor and the
    // reason as the result (FR-008, FR-009).
    let entries = actions_of(&mut client, &lead).await;
    let refused = entries
        .iter()
        .find(|entry| entry["action"] == "refused")
        .expect("the refusal is recorded");
    assert_eq!(
        refused["actor"],
        json!(format!("agent:{lead}")),
        "{refused}"
    );
    assert!(
        refused["result"]
            .as_str()
            .expect("result")
            .contains("itself"),
        "{refused}"
    );

    office.serving.stop().await;
}

/// An agent that asks to start itself over agent-call is refused and the
/// refusal is recorded: start, `new_task`, and stop stay descendants only
/// (FR-008, FR-009; PR 40 MEDIUM-2).
#[tokio::test]
async fn an_agent_starting_itself_over_agent_call_is_refused_and_recorded() {
    assert_an_agent_acting_on_itself_is_refused_and_recorded("agent.start", "start").await;
}

/// An agent that sends itself a new task over agent-call is refused and the
/// refusal is recorded: `new_task` stays descendants only, like stop
/// (FR-008, FR-009; PR 40 MEDIUM-2).
#[tokio::test]
async fn an_agent_tasking_itself_over_agent_call_is_refused_and_recorded() {
    assert_an_agent_acting_on_itself_is_refused_and_recorded("agent.new_task", "new_task").await;
}

/// An agent that asks to stop itself over agent-call is refused and the
/// refusal is recorded (FR-008, FR-009; PR 40 MEDIUM-2).
#[tokio::test]
async fn an_agent_stopping_itself_over_agent_call_is_refused_and_recorded() {
    assert_an_agent_acting_on_itself_is_refused_and_recorded("agent.stop", "stop").await;
}

/// A request with a field the type does not take is refused as `invalid`
/// and recorded, not silently trimmed (PR 40 LOW).
#[tokio::test]
async fn an_agent_stop_with_an_extra_field_is_refused_and_recorded() {
    let office = Office::open().await;
    // A CLI that records the session environment and stays alive, so the
    // test can hold the live credential for its own helper call.
    office.clis.add(
        "lead-cli",
        &calling_script(office.records_path(), "noop", "agent.whoami", "{}"),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place_lead_with_team(&office, &lead, "The Team");
    client.start(&lead).await;
    let (credential, server_file) = session_environment(office.records_path()).await;

    // The extra field rides on an otherwise fine stop of the caller's own
    // id — the strict-field rule refuses before any use case runs.
    let padded = r#"{"agent_id":"x","include_descendants":true}"#;
    let outcome = call_as_session(&credential, &server_file, "agent.stop", padded).await;
    assert!(!outcome.status.success(), "{outcome:?}");
    let answer: Value =
        serde_json::from_str(String::from_utf8_lossy(&outcome.stdout).trim()).expect("json");
    assert_eq!(answer["error"]["code"], "invalid", "{answer}");
    assert!(
        answer["error"]["message"]
            .as_str()
            .expect("message")
            .contains("include_descendants"),
        "{answer}"
    );

    let entries = actions_of(&mut client, &lead).await;
    let refused = entries
        .iter()
        .find(|entry| entry["action"] == "refused")
        .expect("the refusal is recorded");
    assert!(
        refused["result"]
            .as_str()
            .expect("result")
            .contains("include_descendants"),
        "{refused}"
    );

    office.serving.stop().await;
}

/// A lead starts its member over agent-call (FR-008): the answer is the
/// session the start made, the member runs, and the record keeps the start
/// with the lead as the actor.
#[tokio::test]
async fn a_lead_starts_its_member_over_agent_call() {
    let office = Office::open().await;
    office.clis.add("member-cli", "echo ready; sleep 30");
    let request = json!({
        "name": "Bo",
        "command": "member-cli",
        "working_folder": office.work_path(),
    });
    office.clis.add(
        "lead-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &request.to_string(),
        ),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place_lead_with_team(&office, &lead, "The Team");
    client.start(&lead).await;
    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    let member = answer["result"]["id"].as_str().expect("id").to_owned();

    // The lead's live session calls the helper once more, and this time it
    // starts its member.
    let (credential, server_file) = session_environment(office.records_path()).await;
    let start = json!({"agent_id": member}).to_string();
    let outcome = call_as_session(&credential, &server_file, "agent.start", &start).await;
    assert!(outcome.status.success(), "{outcome:?}");
    let started: Value =
        serde_json::from_str(String::from_utf8_lossy(&outcome.stdout).trim()).expect("json");
    assert_eq!(started["ok"], true, "{started}");
    let session = started["result"]["id"]
        .as_str()
        .expect("session id")
        .to_owned();

    // The member runs, and the session the answer named is its own.
    client.wait_for_status(&member, "working").await;
    let sessions = client.sessions(&member).await;
    assert_eq!(sessions.len(), 1, "{sessions:?}");
    assert_eq!(sessions[0]["id"], json!(session), "{sessions:?}");

    // The record keeps the start with the lead as the actor (FR-011).
    let entries = actions_of(&mut client, &member).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "start")
        .expect("the start is recorded");
    assert_eq!(entry["agent_id"], json!(member), "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    assert_eq!(entry["result"], "ok", "{entry}");

    office.serving.stop().await;
}

/// A lead gives its member a new task over agent-call (FR-008, FR-024):
/// the new task starts the member's session, the typed task text lands in
/// it, and the record keeps the new task with the lead as the actor.
#[tokio::test]
async fn a_lead_gives_its_member_a_new_task_over_agent_call() {
    let office = Office::open().await;
    let task = "report to the lead";
    let task_bytes = task.len();
    let typed = office.records_path().join("typed");
    // The member reads exactly the bytes of one typed prompt: the raw mode
    // is set before its first output, so the typing that follows the first
    // output finds the mode ready (the pattern of `sessions.rs`).
    office.clis.add(
        "member-cli",
        &format!(
            "stty raw -echo\necho ready\ndd bs=1 count={task_bytes} of={} 2>/dev/null\nsleep 30",
            typed.display()
        ),
    );
    let request = json!({
        "name": "Bo",
        "command": "member-cli",
        "working_folder": office.work_path(),
    });
    office.clis.add(
        "lead-cli",
        &calling_script(
            office.records_path(),
            "create",
            "agent.create",
            &request.to_string(),
        ),
    );
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Lead", "lead-cli", None).await;
    place_lead_with_team(&office, &lead, "The Team");
    client.start(&lead).await;
    let (code, out) = read_output(office.records_path(), "create").await;
    assert_eq!(code, "0", "{out}");
    let answer: Value = serde_json::from_str(out.trim()).expect("json");
    let member = answer["result"]["id"].as_str().expect("id").to_owned();

    // The user gives the member the text its next session receives as one
    // typed input: a create over agent-call carries no additions.
    let set = client
        .ask(
            "agent.additions.set",
            json!({"agent_id": member, "additions": {"startup_prompt": task}}),
        )
        .await;
    assert_eq!(set["ok"], true, "{set}");

    // The lead's live session calls the helper once more, and this time it
    // gives its member the new task.
    let (credential, server_file) = session_environment(office.records_path()).await;
    let new_task = json!({"agent_id": member}).to_string();
    let outcome = call_as_session(&credential, &server_file, "agent.new_task", &new_task).await;
    assert!(outcome.status.success(), "{outcome:?}");
    let started: Value =
        serde_json::from_str(String::from_utf8_lossy(&outcome.stdout).trim()).expect("json");
    assert_eq!(started["ok"], true, "{started}");
    let session = started["result"]["id"]
        .as_str()
        .expect("session id")
        .to_owned();

    // The member runs its first session, and the task text lands in it
    // typed in, never an argument.
    client.wait_for_status(&member, "working").await;
    let sessions = client.sessions(&member).await;
    assert_eq!(sessions.len(), 1, "{sessions:?}");
    assert_eq!(sessions[0]["id"], json!(session), "{sessions:?}");
    let received = wait_for("the typed task", || {
        fs::read_to_string(&typed)
            .ok()
            .filter(|text| text.len() == task_bytes)
    })
    .await;
    assert_eq!(received, task);

    // The record keeps the new task with the lead as the actor (FR-011).
    let entries = actions_of(&mut client, &member).await;
    let entry = entries
        .iter()
        .find(|entry| entry["action"] == "new_task")
        .expect("the new task is recorded");
    assert_eq!(entry["agent_id"], json!(member), "{entry}");
    assert_eq!(entry["actor"], json!(format!("agent:{lead}")), "{entry}");
    assert_eq!(entry["result"], "ok", "{entry}");

    office.serving.stop().await;
}

/// The agent rows carry the role and the team's display name, read at
/// answer time: a team renamed in storage shows its new name in the next
/// `agent.list`, so the rows never drift from the tree (hierarchy-protocol:
/// `team` is display text, never a key). An agent outside the hierarchy
/// carries null for both. No `team.rename` request exists yet, so the test
/// renames the stored row the way a future rename would write it.
#[tokio::test]
async fn agent_rows_carry_role_and_the_current_team_name() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Bo", "lead-cli", None).await;
    let loner = client.create_agent(&office, "Al", "lone-cli", None).await;
    place(&office, &lead, hierarchy_domain::Role::Lead, None);
    let team = client
        .ask(
            "team.create",
            json!({"name": "Backend", "lead_agent_id": lead}),
        )
        .await;
    assert_eq!(team["ok"], true, "{team}");
    let team_id = team["result"]["id"].as_str().expect("team id").to_owned();

    let rows = list_rows(&mut client).await;
    assert_eq!(rows[&lead]["role"], "lead", "{rows:?}");
    assert_eq!(rows[&lead]["team"], "Backend", "{rows:?}");
    assert_eq!(rows[&loner]["role"], Value::Null, "{rows:?}");
    assert_eq!(rows[&loner]["team"], Value::Null, "{rows:?}");

    let connection =
        rusqlite::Connection::open(office.serving.office.config.database_path()).expect("database");
    connection
        .execute(
            "UPDATE hierarchy_teams SET name = 'Platform', name_key = 'platform' WHERE id = ?1",
            [&team_id],
        )
        .expect("rename");

    let rows = list_rows(&mut client).await;
    assert_eq!(rows[&lead]["team"], "Platform", "{rows:?}");
    assert_eq!(rows[&lead]["role"], "lead", "{rows:?}");

    office.serving.stop().await;
}

/// The `agent.list` rows keyed by agent id.
async fn list_rows(client: &mut Client) -> std::collections::HashMap<String, Value> {
    let listed = client.ask("agent.list", json!({})).await;
    assert_eq!(listed["ok"], true, "{listed}");
    listed["result"]
        .as_array()
        .expect("agent rows")
        .iter()
        .map(|row| (row["id"].as_str().expect("id").to_owned(), row.clone()))
        .collect()
}

/// `team.create` refuses a lead that already leads another team as
/// `conflict` instead of moving it, so the first team never keeps a stale
/// seat and the tree still shows that team with its lead.
#[tokio::test]
async fn a_lead_of_one_team_cannot_create_a_second_team() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let lead = client.create_agent(&office, "Bo", "lead-cli", None).await;
    place(&office, &lead, hierarchy_domain::Role::Lead, None);
    let first = client
        .ask(
            "team.create",
            json!({"name": "Backend", "lead_agent_id": lead}),
        )
        .await;
    assert_eq!(first["ok"], true, "{first}");

    let second = client
        .ask(
            "team.create",
            json!({"name": "Frontend", "lead_agent_id": lead}),
        )
        .await;
    assert_eq!(second["ok"], false, "{second}");
    assert_eq!(second["error"]["code"], "conflict", "{second}");
    assert!(
        second["error"]["message"]
            .as_str()
            .expect("message")
            .contains("the agent already leads another team"),
        "{second}"
    );

    let tree = client.ask("hierarchy.tree", json!({})).await;
    let teams = tree["result"]["teams"].as_array().expect("teams");
    assert_eq!(teams.len(), 1, "{tree}");
    assert_eq!(teams[0]["name"], "Backend", "{tree}");
    assert_eq!(teams[0]["lead"]["id"], json!(lead), "{tree}");

    office.serving.stop().await;
}
