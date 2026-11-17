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
