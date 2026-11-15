// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

mod support;

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use support::running_office::{bind_office, serve_office};
use support::wire_client::{HandshakeFailure, Received, WireClient};
use tower::ServiceExt;
use wire::{AuthMessage, CLOSE_UNAUTHORIZED, ClientHandshake, PRE_AUTH_FRAME_LIMIT};

use office_harnessd::server::ServerOptions;

const SHORT_AUTH_TIMEOUT: Duration = Duration::from_millis(300);
const OTHER_TOKEN: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

fn short_timeout() -> ServerOptions {
    ServerOptions {
        auth_timeout: SHORT_AUTH_TIMEOUT,
        ..ServerOptions::default()
    }
}

/// A connection that has read the challenge and nothing more.
async fn challenged(port: u16) -> (WireClient, String) {
    let mut client = WireClient::connect(port).await.expect("upgrade");
    let Received::Text(challenge) = client.receive().await else {
        panic!("the first frame must be the challenge");
    };
    (client, challenge)
}

#[tokio::test]
async fn a_client_with_the_token_authenticates_and_the_first_frame_is_the_challenge() {
    let serving = serve_office(ServerOptions::default()).await;
    let port = serving.office.port;

    let (_client, challenge) = challenged(port).await;
    let Some(AuthMessage::Challenge { nonce }) = AuthMessage::decode(&challenge) else {
        panic!("not a challenge: {challenge}");
    };
    assert_eq!(nonce.len(), 64);

    let mut client = WireClient::authenticate_ok(port, &serving.office.token()).await;
    let answer = client.request(1, "nonesuch.list").await;
    assert_eq!(answer["id"], 1);
    assert_eq!(answer["ok"], false);
    assert_eq!(answer["error"]["code"], "not_found");
}

#[tokio::test]
async fn a_wrong_proof_is_closed_with_4401_and_nothing_else_is_sent() {
    let serving = serve_office(ServerOptions::default()).await;
    let (mut client, challenge) = challenged(serving.office.port).await;
    let mut handshake = ClientHandshake::new(OTHER_TOKEN, serving.office.port);
    let wrong = handshake.answer_challenge(&challenge).expect("answer");

    client.send_text(&wrong).await;

    assert_eq!(client.expect_close().await, Some(CLOSE_UNAUTHORIZED));
}

#[tokio::test]
async fn a_missing_proof_is_closed_with_4401_after_the_timeout() {
    let serving = serve_office(short_timeout()).await;
    let (mut client, _challenge) = challenged(serving.office.port).await;

    assert_eq!(client.expect_close().await, Some(CLOSE_UNAUTHORIZED));
}

#[tokio::test]
async fn a_late_proof_is_refused() {
    let serving = serve_office(short_timeout()).await;
    let (mut client, challenge) = challenged(serving.office.port).await;
    let mut handshake = ClientHandshake::new(&serving.office.token(), serving.office.port);
    let proof = handshake.answer_challenge(&challenge).expect("answer");
    tokio::time::sleep(SHORT_AUTH_TIMEOUT * 3).await;

    client.send_text(&proof).await;

    assert_eq!(client.expect_close().await, Some(CLOSE_UNAUTHORIZED));
}

#[tokio::test]
async fn any_other_first_frame_is_closed_with_4401_and_gets_no_answer() {
    let serving = serve_office(ServerOptions::default()).await;
    let port = serving.office.port;

    let (mut request_client, _) = challenged(port).await;
    request_client
        .send_text(&json!({"id": 1, "type": "agent.list"}).to_string())
        .await;
    assert_eq!(
        request_client.expect_close().await,
        Some(CLOSE_UNAUTHORIZED)
    );

    let (mut binary_client, _) = challenged(port).await;
    binary_client.send_binary(vec![1, 2, 3]).await;
    assert_eq!(binary_client.expect_close().await, Some(CLOSE_UNAUTHORIZED));

    let (mut garbage_client, _) = challenged(port).await;
    garbage_client.send_text("not json").await;
    assert_eq!(
        garbage_client.expect_close().await,
        Some(CLOSE_UNAUTHORIZED)
    );
}

#[tokio::test]
async fn a_frame_over_one_kib_before_auth_closes_the_socket() {
    let serving = serve_office(ServerOptions::default()).await;
    let (mut client, _) = challenged(serving.office.port).await;
    let padding = "a".repeat(PRE_AUTH_FRAME_LIMIT + 1);

    client.send_text(&padding).await;

    assert_eq!(client.expect_close().await, Some(CLOSE_UNAUTHORIZED));
}

#[tokio::test]
async fn an_upgrade_with_a_wrong_host_or_any_origin_is_refused() {
    let serving = serve_office(ServerOptions::default()).await;
    let port = serving.office.port;
    let wrong_hosts = ["localhost", "evil.example", "127.0.0.1:1"];
    let origins = ["http://127.0.0.1", "http://evil.example", "null"];

    for host in wrong_hosts {
        let refused = WireClient::connect_with_headers(port, &[("Host", host)]).await;
        assert_refused_with_403(refused, host);
    }
    for origin in origins {
        let refused = WireClient::connect_with_headers(port, &[("Origin", origin)]).await;
        assert_refused_with_403(refused, origin);
    }
}

fn assert_refused_with_403(
    outcome: Result<WireClient, tokio_tungstenite::tungstenite::Error>,
    what: &str,
) {
    match outcome {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{what}");
            assert!(response.body().as_ref().is_none_or(Vec::is_empty), "{what}");
        }
        Err(other) => panic!("{what}: expected an HTTP refusal, got {other}"),
        Ok(_) => panic!("{what}: the upgrade must be refused"),
    }
}

#[tokio::test]
async fn a_proof_computed_for_another_port_is_refused() {
    let serving = serve_office(ServerOptions::default()).await;
    let port = serving.office.port;

    let outcome =
        WireClient::authenticate(port, port.wrapping_add(1), &serving.office.token()).await;

    assert!(
        matches!(
            outcome,
            Err(HandshakeFailure::Refused(Some(CLOSE_UNAUTHORIZED)))
        ),
        "{:?}",
        outcome.err()
    );
}

#[tokio::test]
async fn the_socket_is_bound_to_loopback_only() {
    let office = bind_office(ServerOptions::default()).await;
    let server = office.server.as_ref().expect("server");

    assert_eq!(server.address().ip(), Ipv4Addr::LOCALHOST);
    assert!(server.address().ip().is_loopback());
    assert_ne!(server.port(), 0);
}

#[tokio::test]
async fn a_peer_that_is_not_loopback_is_refused_and_a_loopback_peer_is_served() {
    let office = bind_office(ServerOptions::default()).await;
    let router = office.server.as_ref().expect("server").router();
    let outside = SocketAddr::from(([203, 0, 113, 9], 50_000));
    let inside = SocketAddr::from(([127, 0, 0, 1], 50_000));

    for path in ["/v1/health", "/v1/ws"] {
        let refused = router
            .clone()
            .oneshot(request_from(path, outside))
            .await
            .expect("response");
        assert_eq!(refused.status(), StatusCode::FORBIDDEN, "{path}");
        let body = to_bytes(refused.into_body(), usize::MAX)
            .await
            .expect("body");
        assert!(body.is_empty(), "{path}");
    }
    let served = router
        .oneshot(request_from("/v1/health", inside))
        .await
        .expect("response");
    assert_eq!(served.status(), StatusCode::OK);
}

fn request_from(path: &str, peer: SocketAddr) -> Request<Body> {
    let mut request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("request");
    request.extensions_mut().insert(ConnectInfo(peer));
    request
}

#[tokio::test]
async fn health_needs_no_auth_and_reveals_nothing_about_the_office() {
    let office = bind_office(ServerOptions::default()).await;
    let router = office.server.as_ref().expect("server").router();
    let peer = SocketAddr::from(([127, 0, 0, 1], 50_000));

    let response = router
        .oneshot(request_from("/v1/health", peer))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let health: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(health, json!({"ok": true, "protocol": 1}));
}

#[tokio::test]
async fn server_json_names_the_port_and_the_start() {
    let office = bind_office(ServerOptions::default()).await;

    let text = std::fs::read_to_string(office.server_file_path()).expect("server.json");

    let file: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(file["port"], office.port);
    assert_eq!(file["protocol"], 1);
    assert_eq!(file["pid"], std::process::id());
    assert_eq!(file["start_nonce"].as_str().map(str::len), Some(64));
    assert!(
        !text.contains(&office.token()),
        "server.json holds no token"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn the_token_and_server_json_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let office = bind_office(ServerOptions::default()).await;
    let mode = |path: &std::path::Path| {
        std::fs::metadata(path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    };

    assert_eq!(mode(&office.config.token_path()), 0o600);
    assert_eq!(mode(&office.server_file_path()), 0o600);
    assert_eq!(office.token().len(), 64);
}

#[tokio::test]
async fn after_regeneration_the_old_token_fails_and_its_open_sockets_close() {
    let serving = serve_office(ServerOptions::default()).await;
    let port = serving.office.port;
    let old_token = serving.office.token();
    let mut caller = WireClient::authenticate_ok(port, &old_token).await;
    let mut bystander = WireClient::authenticate_ok(port, &old_token).await;

    let answer = caller.request(9, "token.regenerate").await;

    assert_eq!(answer, json!({"id": 9, "ok": true, "result": null}));
    assert_eq!(
        caller.expect_close().await,
        Some(CLOSE_UNAUTHORIZED),
        "the caller closes too"
    );
    assert_eq!(bystander.expect_close().await, Some(CLOSE_UNAUTHORIZED));
    let new_token = serving.office.token();
    assert_ne!(new_token, old_token);
    let with_old = WireClient::authenticate(port, port, &old_token).await;
    assert!(matches!(
        with_old,
        Err(HandshakeFailure::Refused(Some(CLOSE_UNAUTHORIZED)))
    ));
    let mut with_new = WireClient::authenticate_ok(port, &new_token).await;
    assert_eq!(with_new.request(1, "agent.list").await["id"], 1);
}

#[tokio::test]
async fn the_token_never_appears_in_the_log_or_in_any_answer() {
    let serving = serve_office(ServerOptions::default()).await;
    let port = serving.office.port;
    let first_token = serving.office.token();
    let mut seen_text = Vec::new();

    let _ = WireClient::authenticate(port, port, OTHER_TOKEN).await;
    let _ = WireClient::connect_with_headers(port, &[("Origin", "http://evil.example")]).await;
    let mut client = WireClient::authenticate_ok(port, &first_token).await;
    seen_text.push(client.request(1, "agent.list").await.to_string());
    seen_text.push(client.request(2, "token.show").await.to_string());
    client.send_text("{broken").await;
    seen_text.push(client.receive_json().await.to_string());
    seen_text.push(client.request(3, "token.regenerate").await.to_string());
    let second_token = serving.office.token();

    let log = serving.office.log_text();

    assert!(
        log.contains("connection refused"),
        "the log records the refusals: {log}"
    );
    for secret in [&first_token, &second_token, &OTHER_TOKEN.to_owned()] {
        assert!(!log.contains(secret.as_str()), "the log holds a token");
        for text in &seen_text {
            assert!(
                !text.contains(secret.as_str()),
                "an answer holds a token: {text}"
            );
        }
    }
}

#[tokio::test]
async fn storage_errors_reach_authenticated_clients_as_events() {
    use agent_infrastructure::persistence::PersistenceError;
    use rusqlite::ffi;

    let serving = serve_office(ServerOptions::default()).await;
    let mut client =
        WireClient::authenticate_ok(serving.office.port, &serving.office.token()).await;
    let disk_full = rusqlite::Error::SqliteFailure(ffi::Error::new(ffi::SQLITE_FULL), None);

    let guarded: Result<(), _> = serving
        .office
        .services
        .storage_alerts
        .guard(Err(PersistenceError::Storage(disk_full)));

    assert!(guarded.is_err());
    let event = client.receive_json().await;
    assert_eq!(event["type"], "storage.error");
    assert!(
        event["message"]
            .as_str()
            .is_some_and(|message| !message.is_empty())
    );
    assert!(event.get("id").is_none());
}

#[tokio::test]
async fn stopping_the_server_closes_connections_and_removes_server_json() {
    let serving = serve_office(ServerOptions::default()).await;
    let mut client =
        WireClient::authenticate_ok(serving.office.port, &serving.office.token()).await;
    let server_file = serving.office.server_file_path();
    assert!(server_file.exists());

    let _office = serving.stop().await;

    assert_eq!(client.expect_close().await, Some(1001));
    assert!(!server_file.exists());
}

#[tokio::test]
async fn a_connection_without_a_free_handshake_slot_is_closed_with_4401() {
    let options = ServerOptions {
        max_concurrent_handshakes: 1,
        ..ServerOptions::default()
    };
    let serving = serve_office(options).await;
    let port = serving.office.port;
    let (holder, _challenge) = challenged(port).await;

    let mut refused = WireClient::connect(port).await.expect("upgrade");

    assert_eq!(refused.expect_close().await, Some(CLOSE_UNAUTHORIZED));
    drop(holder);
}
