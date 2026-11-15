// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]
#![cfg(unix)]

//! The `terminal.*` requests and binary frames through a real socket, with a
//! shell as the child.

mod support;

use std::time::Duration;

use kernel::AgentId;
use serde_json::json;
use support::running_office::serve_office;
use support::wire_client::{Received, WireClient};
use terminal_application::common::models::SpawnTerminalRequest;
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use office_harnessd::server::ServerOptions;

const SNAPSHOT_KIND: u8 = 0x01;
const HEADER_BYTES: usize = 17;
const GRACE: Duration = Duration::from_millis(300);
const CLOSE_INTERNAL_ERROR: u16 = 1011;
const OVERFLOW_WAIT: Duration = Duration::from_secs(5);
const FLOOD_LINE_BYTES: usize = 3000;
const DRAIN_LIMIT: Duration = Duration::from_secs(30);

fn uuid_hex(agent: AgentId) -> String {
    agent.to_string().replace('-', "")
}

fn header_names(frame: &[u8], agent: AgentId) -> bool {
    let hex = uuid_hex(agent);
    frame[1..HEADER_BYTES]
        .iter()
        .enumerate()
        .all(|(index, byte)| u8::from_str_radix(&hex[2 * index..2 * index + 2], 16) == Ok(*byte))
}

fn start_shell(office: &support::running_office::RunningOffice, agent: AgentId, script: &str) {
    office
        .services
        .terminal
        .runtime
        .spawn(&SpawnTerminalRequest {
            agent_id: agent,
            program: "/bin/sh".to_owned(),
            arguments: vec!["-c".to_owned(), script.to_owned()],
            working_folder: std::env::temp_dir(),
            environment: vec![("PATH".to_owned(), "/usr/bin:/bin".to_owned())],
            size: TerminalSize::new(80, 24).expect("size"),
            output_key: OutputKey::parse(&agent.to_string()).expect("key"),
        })
        .expect("spawn");
}

/// Reads binary frames until the text of their payloads holds `needle`.
async fn read_until(client: &mut WireClient, agent: AgentId, needle: &str) -> Vec<u8> {
    let mut text = String::new();
    let mut first = None;
    while !text.contains(needle) {
        let frame = client.receive_binary().await;
        assert!(header_names(&frame, agent));
        text.push_str(&String::from_utf8_lossy(&frame[HEADER_BYTES..]));
        first.get_or_insert(frame);
    }
    first.expect("a frame")
}

#[tokio::test]
async fn attach_sends_a_snapshot_then_live_frames_and_input_reaches_the_child() {
    let serving = serve_office(ServerOptions::default()).await;
    let agent = AgentId::generate();
    start_shell(&serving.office, agent, "echo ready; cat");
    let mut client =
        WireClient::authenticate_ok(serving.office.port, &serving.office.token()).await;

    let attached = client
        .request_with(json!({"id": 1, "type": "terminal.attach", "agent_id": agent.to_string(), "cols": 80, "rows": 24}))
        .await;
    assert_eq!(attached["ok"], true);
    let first = read_until(&mut client, agent, "ready").await;
    assert_eq!(first[0], SNAPSHOT_KIND, "the snapshot comes first");

    // "hello\n" in base64.
    let typed = client
        .request_with(json!({"id": 2, "type": "terminal.input", "agent_id": agent.to_string(), "data": "aGVsbG8K"}))
        .await;
    assert_eq!(typed["ok"], true);
    read_until(&mut client, agent, "hello").await;

    let resized = client
        .request_with(json!({"id": 3, "type": "terminal.resize", "agent_id": agent.to_string(), "cols": 100, "rows": 30}))
        .await;
    assert_eq!(resized["ok"], true);
    let detached = client
        .request_with(json!({"id": 4, "type": "terminal.detach", "agent_id": agent.to_string()}))
        .await;
    assert_eq!(detached["ok"], true);

    serving
        .office
        .services
        .terminal
        .runtime
        .stop(agent, GRACE)
        .expect("stop");
    serving.stop().await;
}

#[tokio::test]
async fn bad_fields_and_unknown_agents_get_named_errors() {
    let serving = serve_office(ServerOptions::default()).await;
    let mut client =
        WireClient::authenticate_ok(serving.office.port, &serving.office.token()).await;
    let agent = AgentId::generate().to_string();

    let zero = client
        .request_with(
            json!({"id": 1, "type": "terminal.resize", "agent_id": agent, "cols": 0, "rows": 24}),
        )
        .await;
    assert_eq!(zero["error"]["code"], "invalid");
    assert!(
        zero["error"]["message"]
            .as_str()
            .expect("text")
            .contains("cols")
    );

    let missing = client
        .request_with(json!({"id": 2, "type": "terminal.attach", "agent_id": agent, "rows": 24}))
        .await;
    assert_eq!(missing["error"]["code"], "invalid");
    assert!(
        missing["error"]["message"]
            .as_str()
            .expect("text")
            .contains("cols")
    );

    let not_an_id = client
        .request_with(json!({"id": 3, "type": "terminal.input", "agent_id": "nope", "data": ""}))
        .await;
    assert!(
        not_an_id["error"]["message"]
            .as_str()
            .expect("text")
            .contains("agent_id")
    );

    let not_base64 = client
        .request_with(json!({"id": 4, "type": "terminal.input", "agent_id": agent, "data": "***"}))
        .await;
    assert!(
        not_base64["error"]["message"]
            .as_str()
            .expect("text")
            .contains("data")
    );

    let unknown = client
        .request_with(
            json!({"id": 5, "type": "terminal.attach", "agent_id": agent, "cols": 80, "rows": 24}),
        )
        .await;
    assert_eq!(unknown["error"]["code"], "not_found");

    let other = client.request(6, "terminal.nothing").await;
    assert_eq!(other["error"]["code"], "not_found");
    serving.stop().await;
}

#[tokio::test]
async fn a_client_that_stops_reading_is_disconnected_with_1011_so_it_can_attach_again() {
    let serving = serve_office(ServerOptions::default()).await;
    let agent = AgentId::generate();
    start_shell(
        &serving.office,
        agent,
        &format!("exec yes {}", "x".repeat(FLOOD_LINE_BYTES)),
    );
    let mut client =
        WireClient::authenticate_ok(serving.office.port, &serving.office.token()).await;
    let attached = client
        .request_with(json!({"id": 1, "type": "terminal.attach", "agent_id": agent.to_string(), "cols": 80, "rows": 24}))
        .await;
    assert_eq!(attached["ok"], true);

    // The client reads nothing while the child floods the terminal.
    tokio::time::sleep(OVERFLOW_WAIT).await;

    let deadline = tokio::time::Instant::now() + DRAIN_LIMIT;
    let closed = loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the server never closed the connection"
        );
        if let Received::Closed(code) = client.receive().await {
            break code;
        }
    };
    // A close frame may be lost when the peer's buffers are full; then the
    // stream just ends.
    assert!(
        closed.is_none() || closed == Some(CLOSE_INTERNAL_ERROR),
        "{closed:?}"
    );

    let mut again = WireClient::authenticate_ok(serving.office.port, &serving.office.token()).await;
    let attached = again
        .request_with(json!({"id": 2, "type": "terminal.attach", "agent_id": agent.to_string(), "cols": 80, "rows": 24}))
        .await;
    assert_eq!(attached["ok"], true, "a fresh connection attaches again");
    serving
        .office
        .services
        .terminal
        .runtime
        .stop(agent, GRACE)
        .expect("stop");
    serving.stop().await;
}
