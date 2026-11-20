// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! The subtree behavior at protocol level: a stop or delete of a parent
//! covers its subtree through the bridge the bootstrap installed, with one
//! event per affected agent, deepest first (FR-006, FR-007).

mod support;

use std::sync::Arc;

use agent_infrastructure::persistence::SharedConnection;
use hierarchy_application::commands::place_agent::{PlaceAgentHandler, PlaceAgentRequest};
use hierarchy_application::common::interfaces::HierarchyStore;
use hierarchy_domain::Role;
use hierarchy_infrastructure::persistence::repositories::hierarchy_repository::HierarchyRepository;
use hierarchy_infrastructure::services::agent_places::AgentPlaces;

use office_harnessd::server::ServerOptions;
use serde_json::{Value, json};
use support::fake_cli::FakeClis;
use support::running_office::{ServingOffice, serve_office_with};
use support::wire_client::WireClient;

const QUIET_PERIOD_MS: u64 = 400;

struct Office {
    serving: ServingOffice,
    clis: FakeClis,
    work_folder: tempfile::TempDir,
}

impl Office {
    async fn open() -> Self {
        let clis = FakeClis::new();
        let serving = serve_office_with(
            ServerOptions::default(),
            clis.login_setup(),
            Some(QUIET_PERIOD_MS),
        )
        .await;
        Self {
            serving,
            clis,
            work_folder: tempfile::tempdir().expect("work folder"),
        }
    }

    async fn client(&self) -> WireClient {
        WireClient::authenticate_ok(self.serving.office.port, &self.serving.office.token()).await
    }

    fn work_path(&self) -> &str {
        self.work_folder.path().to_str().expect("utf-8 path")
    }
}

/// Sends a request and returns its answer, skipping events that arrive first.
async fn call(client: &mut WireClient, id: u32, kind: &str, fields: Value) -> Value {
    let mut body = fields;
    body["id"] = json!(id);
    body["type"] = json!(kind);
    client.send_text(&body.to_string()).await;
    loop {
        let message = client.receive_json().await;
        if message["id"] == json!(id) {
            return message;
        }
    }
}

fn error_text(answer: &Value) -> (&str, &str) {
    (
        answer["error"]["code"].as_str().expect("code"),
        answer["error"]["message"].as_str().expect("message"),
    )
}

/// Reads events until one of the type `kind` names the agent. Returns it.
async fn next_event(client: &mut WireClient, kind: &str, agent_id: &str) -> Value {
    loop {
        let message = client.receive_json().await;
        if message["type"] == kind && message["agent_id"] == json!(agent_id) {
            return message;
        }
    }
}

/// The statuses of each agent, in order, until every agent in `finals` has
/// reached its last status.
async fn statuses_until(
    client: &mut WireClient,
    finals: &[(&str, &str)],
) -> std::collections::HashMap<String, Vec<String>> {
    let mut seen: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let done = |seen: &std::collections::HashMap<String, Vec<String>>| {
        finals.iter().all(|(agent, last)| {
            seen.get(*agent)
                .and_then(|list| list.last())
                .is_some_and(|status| status == last)
        })
    };
    while !done(&seen) {
        let message = client.receive_json().await;
        if message["type"] == "agent.status" {
            let agent = message["agent_id"].as_str().expect("agent").to_owned();
            let status = message["status"].as_str().expect("status").to_owned();
            seen.entry(agent).or_default().push(status);
        }
    }
    seen
}

async fn create_agent(
    office: &Office,
    client: &mut WireClient,
    id: u32,
    name: &str,
    command: &str,
) -> String {
    let answer = call(
        client,
        id,
        "agent.create",
        json!({"name": name, "command": command, "working_folder": office.work_path()}),
    )
    .await;
    assert_eq!(answer["ok"], true, "{answer}");
    answer["result"]["id"].as_str().expect("id").to_owned()
}

/// Places an agent through the same store the server built, so the tree
/// rules see the arrangement. No wire request places a member yet (the
/// `agent.create` role flow is 0002 work in progress), so the test writes
/// the place directly.
fn place(
    office: &support::running_office::RunningOffice,
    agent_id: &str,
    role: Role,
    parent_agent_id: Option<&str>,
) {
    let connection = rusqlite::Connection::open(office.config.database_path()).expect("database");
    let store: Arc<dyn HierarchyStore> = Arc::new(HierarchyRepository::new(
        SharedConnection::new(connection).handle(),
        AgentPlaces::new(),
    ));
    PlaceAgentHandler::new(store)
        .handle(PlaceAgentRequest {
            agent_id: agent_id.parse().expect("agent id"),
            role,
            parent_agent_id: parent_agent_id.map(|id| id.parse().expect("agent id")),
            team_id: None,
        })
        .expect("placement");
}

/// The descendants the wire names for an agent.
async fn descendant_count(client: &mut WireClient, agent_id: &str) -> u64 {
    let counted = call(
        client,
        90,
        "hierarchy.descendants",
        json!({"agent_id": agent_id}),
    )
    .await;
    assert_eq!(counted["ok"], true, "{counted}");
    counted["result"]["count"].as_u64().expect("count")
}

#[tokio::test]
async fn a_stop_and_a_delete_of_a_parent_over_the_wire_cover_the_subtree() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let mut observer = office.client().await;
    let ceo = create_agent(&office, &mut client, 1, "Ceo", "fake-cli").await;
    let lead = create_agent(&office, &mut client, 2, "Lead", "fake-cli").await;
    place(&office.serving.office, &ceo, Role::Ceo, None);
    place(&office.serving.office, &lead, Role::Lead, Some(&ceo));
    assert_eq!(descendant_count(&mut client, &ceo).await, 1);

    for (id, agent) in [(3, &ceo), (4, &lead)] {
        let started = call(&mut client, id, "agent.start", json!({"agent_id": agent})).await;
        assert_eq!(started["ok"], true, "{started}");
    }
    statuses_until(&mut observer, &[(&ceo, "working"), (&lead, "working")]).await;

    let stopped = call(
        &mut client,
        5,
        "agent.stop",
        json!({"agent_id": ceo, "include_descendants": true}),
    )
    .await;
    assert_eq!(stopped["ok"], true, "{stopped}");
    // The lead stops first, the CEO last (FR-007).
    let lead_stopped = next_event(&mut observer, "agent.status", &lead).await;
    assert_eq!(lead_stopped["status"], "stopped");
    let ceo_stopped = next_event(&mut observer, "agent.status", &ceo).await;
    assert_eq!(ceo_stopped["status"], "stopped");

    let deleted = call(
        &mut client,
        6,
        "agent.delete",
        json!({
            "agent_id": ceo,
            "include_descendants": true,
            "descendant_count": 1
        }),
    )
    .await;
    assert_eq!(deleted["ok"], true, "{deleted}");
    // The delete runs deepest first too (FR-006), one event per agent.
    let lead_deleted = next_event(&mut observer, "agent.deleted", &lead).await;
    assert_eq!(lead_deleted["agent_id"], lead);
    let ceo_deleted = next_event(&mut observer, "agent.deleted", &ceo).await;
    assert_eq!(ceo_deleted["agent_id"], ceo);

    let listed = call(&mut client, 7, "agent.list", json!({})).await;
    assert_eq!(listed["result"].as_array().expect("agents").len(), 0);
    assert_eq!(descendant_count(&mut client, &ceo).await, 0);
    office.serving.stop().await;
}

#[tokio::test]
async fn a_delete_of_a_parent_without_the_count_conflicts_and_deletes_nothing() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let ceo = create_agent(&office, &mut client, 1, "Ceo", "x").await;
    let lead = create_agent(&office, &mut client, 2, "Lead", "x").await;
    place(&office.serving.office, &ceo, Role::Ceo, None);
    place(&office.serving.office, &lead, Role::Lead, Some(&ceo));

    let refused = call(
        &mut client,
        3,
        "agent.delete",
        json!({"agent_id": ceo, "include_descendants": true}),
    )
    .await;

    let (code, message) = error_text(&refused);
    assert_eq!(code, "conflict", "{refused}");
    assert!(message.contains("descendant_count 1"), "{message}");
    let listed = call(&mut client, 4, "agent.list", json!({})).await;
    assert_eq!(listed["result"].as_array().expect("agents").len(), 2);
    office.serving.stop().await;
}

/// `agent.create` with a `role` object creates the agent and gives it its
/// place in one step (0002, FR-003): a lead with its team shows in the
/// tree, and a member under a lead takes the lead's team (A6). A second CEO
/// is refused and leaves nothing behind.
#[tokio::test]
async fn an_agent_create_with_a_role_places_the_agent_in_one_step() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let _plain = create_agent(&office, &mut client, 1, "Ceo", "fake-cli").await;
    let chief = call(
        &mut client,
        2,
        "agent.create",
        json!({
            "name": "Chief",
            "command": "fake-cli",
            "working_folder": office.work_path(),
            "role": {"kind": "ceo"},
        }),
    )
    .await;
    assert_eq!(chief["ok"], true, "{chief}");
    let chief_id = chief["result"]["id"].as_str().expect("id").to_owned();

    // A lead with its team: the user's team.create carries the lead agent
    // to pick, so the test picks the agent it just created. The tree shows
    // lead and team together.
    let plain_lead = create_agent(&office, &mut client, 3, "Ana", "fake-cli").await;
    let team = call(
        &mut client,
        4,
        "team.create",
        json!({"name": "The Team", "lead_agent_id": plain_lead}),
    )
    .await;
    assert_eq!(team["ok"], true, "{team}");
    let lead_id = plain_lead;

    // A member under the lead, no team sent: the lead's team is its team
    // (A6), and the create-and-place is one request (FR-003).
    let member = call(
        &mut client,
        5,
        "agent.create",
        json!({
            "name": "Bo",
            "command": "fake-cli",
            "working_folder": office.work_path(),
            "role": {"kind": "member", "parent_agent_id": lead_id},
        }),
    )
    .await;
    assert_eq!(member["ok"], true, "{member}");
    let member_id = member["result"]["id"].as_str().expect("id").to_owned();

    let tree = call(&mut client, 6, "hierarchy.tree", json!({})).await;
    assert_eq!(tree["ok"], true, "{tree}");
    assert_eq!(tree["result"]["ceo"]["id"], json!(chief_id), "{tree}");
    let teams = tree["result"]["teams"].as_array().expect("teams");
    assert_eq!(teams.len(), 1, "{tree}");
    assert_eq!(teams[0]["lead"]["id"], json!(lead_id));
    let members = teams[0]["members"].as_array().expect("members");
    assert_eq!(members.len(), 1, "{tree}");
    assert_eq!(members[0]["id"], json!(member_id));
    assert_eq!(members[0]["role"], "member");

    // A second CEO is refused and creates nothing (FR-003).
    let second = call(
        &mut client,
        7,
        "agent.create",
        json!({
            "name": "Sid",
            "command": "fake-cli",
            "working_folder": office.work_path(),
            "role": {"kind": "ceo"},
        }),
    )
    .await;
    let (code, message) = error_text(&second);
    assert_eq!(code, "invalid", "{second}");
    assert!(message.contains("already has a CEO"), "{message}");
    let listed = call(&mut client, 8, "agent.list", json!({})).await;
    let names: Vec<&str> = listed["result"]
        .as_array()
        .expect("agents")
        .iter()
        .filter_map(|agent| agent["name"].as_str())
        .collect();
    assert!(!names.contains(&"Sid"), "{names:?}");
    office.serving.stop().await;
}

/// The contract's `lead` object over the real server, as the desktop's team
/// form sends it (0002 hierarchy-protocol, `team.create`): the lead is
/// created and leads the new team in one request. A taken team name creates
/// no lead, and a lead object with a field it does not take is invalid.
#[tokio::test]
async fn a_team_create_with_a_lead_object_creates_the_lead_and_the_team() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let lead = json!({
        "name": "Lea",
        "command": "fake-cli",
        "working_folder": office.work_path(),
    });

    let team = call(
        &mut client,
        1,
        "team.create",
        json!({"name": "Api", "lead": lead}),
    )
    .await;
    assert_eq!(team["ok"], true, "{team}");
    let team_id = team["result"]["id"].as_str().expect("team id").to_owned();
    let tree = call(&mut client, 2, "hierarchy.tree", json!({})).await;
    let teams = tree["result"]["teams"].as_array().expect("teams");
    assert_eq!(teams.len(), 1, "{tree}");
    assert_eq!(teams[0]["id"], team_id.as_str(), "{tree}");
    assert_eq!(teams[0]["lead"]["role"], "lead", "{tree}");
    assert_eq!(teams[0]["lead"]["team_id"], team_id.as_str(), "{tree}");

    let taken = call(
        &mut client,
        3,
        "team.create",
        json!({"name": "api", "lead": {"name": "Leo", "command": "fake-cli", "working_folder": office.work_path()}}),
    )
    .await;
    assert_eq!(error_text(&taken).0, "conflict", "{taken}");
    let agents = call(&mut client, 4, "agent.list", json!({})).await;
    let names: Vec<&str> = agents["result"]
        .as_array()
        .expect("agents")
        .iter()
        .filter_map(|agent| agent["name"].as_str())
        .collect();
    assert_eq!(
        names,
        vec!["Lea"],
        "a refused team leaves no lead: {agents}"
    );
    let lea = agents["result"][0]["id"].as_str().expect("lead id");
    assert_eq!(teams[0]["lead"]["id"], lea, "{tree}");

    let extra = call(
        &mut client,
        5,
        "team.create",
        json!({"name": "Web", "lead": {"name": "Lu", "command": "fake-cli", "role": "ceo"}}),
    )
    .await;
    assert_eq!(error_text(&extra).0, "invalid", "{extra}");
    let both = call(
        &mut client,
        6,
        "team.create",
        json!({"name": "Web", "lead": lead, "lead_agent_id": team_id}),
    )
    .await;
    assert_eq!(error_text(&both).0, "invalid", "{both}");
    office.serving.stop().await;
}

/// Reads events until one of the type `kind` arrives. Returns it.
async fn next_event_of(client: &mut WireClient, kind: &str) -> Value {
    loop {
        let message = client.receive_json().await;
        if message["type"] == kind {
            return message;
        }
    }
}

#[tokio::test]
async fn a_team_create_with_a_lead_object_tells_every_client_of_the_new_lead() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let mut watcher = office.client().await;
    let lead = json!({
        "name": "Lea",
        "command": "fake-cli",
        "working_folder": office.work_path(),
    });

    let team = call(
        &mut client,
        1,
        "team.create",
        json!({"name": "Api", "lead": lead}),
    )
    .await;
    assert_eq!(team["ok"], true, "{team}");
    let lead_id = team["result"]["lead_agent_id"].as_str().expect("lead id");

    let changed = next_event_of(&mut watcher, "agent.changed").await;
    assert_eq!(changed["agent"]["id"], lead_id, "{changed}");
    assert_eq!(changed["agent"]["name"], "Lea", "{changed}");
    let tree = next_event_of(&mut watcher, "hierarchy.changed").await;
    assert_eq!(tree["cause"], "team.created", "{tree}");
    let recorded = next_event_of(&mut watcher, "action.recorded").await;
    assert_eq!(recorded["entry"]["agent_id"], lead_id, "{recorded}");
    assert_eq!(recorded["entry"]["action"], "create", "{recorded}");

    // Refused team creates make no agent, so the next agent.changed the
    // watcher sees is the one of the agent created after them.
    let taken_team = call(
        &mut client,
        2,
        "team.create",
        json!({"name": "api", "lead": {"name": "Leo", "command": "fake-cli", "working_folder": office.work_path()}}),
    )
    .await;
    assert_eq!(error_text(&taken_team).0, "conflict", "{taken_team}");
    let taken_lead = call(
        &mut client,
        3,
        "team.create",
        json!({"name": "Web", "lead": lead}),
    )
    .await;
    assert_eq!(error_text(&taken_lead).0, "conflict", "{taken_lead}");
    let marker = call(
        &mut client,
        4,
        "agent.create",
        json!({"name": "Marker", "command": "fake-cli", "working_folder": office.work_path()}),
    )
    .await;
    assert_eq!(marker["ok"], true, "{marker}");
    let next_changed = next_event_of(&mut watcher, "agent.changed").await;
    assert_eq!(next_changed["agent"]["name"], "Marker", "{next_changed}");
    office.serving.stop().await;
}

/// Creates the CEO through `agent.create` with a `role` object.
async fn create_ceo(office: &Office, client: &mut WireClient, id: u32) -> String {
    let created = call(
        client,
        id,
        "agent.create",
        json!({
            "name": "Chief",
            "command": "fake-cli",
            "working_folder": office.work_path(),
            "role": {"kind": "ceo"},
        }),
    )
    .await;
    assert_eq!(created["ok"], true, "{created}");
    created["result"]["id"].as_str().expect("id").to_owned()
}

/// A CEO with one lead under it, both placed and both running.
async fn started_ceo_with_lead(
    office: &Office,
    client: &mut WireClient,
    observer: &mut WireClient,
) -> (String, String) {
    let ceo = create_agent(office, client, 1, "Ceo", "fake-cli").await;
    let lead = create_agent(office, client, 2, "Lead", "fake-cli").await;
    place(&office.serving.office, &ceo, Role::Ceo, None);
    place(&office.serving.office, &lead, Role::Lead, Some(&ceo));
    for (id, agent) in [(3, &ceo), (4, &lead)] {
        let started = call(client, id, "agent.start", json!({"agent_id": agent})).await;
        assert_eq!(started["ok"], true, "{started}");
    }
    statuses_until(observer, &[(&ceo, "working"), (&lead, "working")]).await;
    (ceo, lead)
}

#[tokio::test]
async fn an_agent_create_with_a_role_tells_every_client_the_tree_changed() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let mut watcher = office.client().await;

    let ceo = create_ceo(&office, &mut client, 1).await;

    // The new agent goes first, so a client knows it before the tree names it.
    let changed = next_event_of(&mut watcher, "agent.changed").await;
    assert_eq!(changed["agent"]["id"], ceo.as_str(), "{changed}");
    let tree = next_event_of(&mut watcher, "hierarchy.changed").await;
    assert_eq!(tree["cause"], "agent.placed", "{tree}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_delete_of_a_placed_agent_tells_every_client_the_tree_changed() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let mut watcher = office.client().await;
    let ceo = create_ceo(&office, &mut client, 1).await;
    let placed = next_event_of(&mut watcher, "hierarchy.changed").await;
    assert_eq!(placed["cause"], "agent.placed", "{placed}");

    let deleted = call(&mut client, 2, "agent.delete", json!({"agent_id": ceo})).await;

    assert_eq!(deleted["ok"], true, "{deleted}");
    let tree = next_event_of(&mut watcher, "hierarchy.changed").await;
    assert_eq!(tree["cause"], "agent.unplaced", "{tree}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_subtree_stop_tells_every_client_the_tree_changed() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let mut watcher = office.client().await;
    let (ceo, _lead) = started_ceo_with_lead(&office, &mut client, &mut watcher).await;

    let stopped = call(
        &mut client,
        5,
        "agent.stop",
        json!({"agent_id": ceo, "include_descendants": true}),
    )
    .await;

    assert_eq!(stopped["ok"], true, "{stopped}");
    let tree = next_event_of(&mut watcher, "hierarchy.changed").await;
    assert_eq!(tree["cause"], "subtree.stopped", "{tree}");
    office.serving.stop().await;
}

#[tokio::test]
async fn a_subtree_delete_tells_every_client_the_tree_changed() {
    let office = Office::open().await;
    office.clis.add("fake-cli", "echo up; sleep 30");
    let mut client = office.client().await;
    let mut watcher = office.client().await;
    let (ceo, _lead) = started_ceo_with_lead(&office, &mut client, &mut watcher).await;

    let deleted = call(
        &mut client,
        5,
        "agent.delete",
        json!({"agent_id": ceo, "include_descendants": true, "descendant_count": 1}),
    )
    .await;

    assert_eq!(deleted["ok"], true, "{deleted}");
    let tree = next_event_of(&mut watcher, "hierarchy.changed").await;
    assert_eq!(tree["cause"], "subtree.deleted", "{tree}");
    office.serving.stop().await;
}
