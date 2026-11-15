// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

mod support;

use futures_util::{SinkExt, StreamExt};
use support::wire_client::{HandshakeFailure, WireClient};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use wire::{AuthMessage, ProofRole, compute_proof};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const NONCE: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
const WRONG_TOKEN: &str = "abababababababababababababababababababababababababababababababab";

/// A server that cannot produce the server proof: it knows another token. It
/// returns every frame it received after the client's proof.
async fn fake_server_with_token(
    known_token: &'static str,
) -> (u16, tokio::task::JoinHandle<Vec<Message>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("address").port();
    let task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.expect("accept");
        let mut socket = accept_async(tcp).await.expect("upgrade");
        let challenge = AuthMessage::Challenge {
            nonce: NONCE.to_owned(),
        };
        socket
            .send(Message::text(challenge.encode()))
            .await
            .expect("challenge");
        let _client_proof = socket.next().await.expect("client proof").expect("frame");
        let proof = compute_proof(known_token, NONCE, port, ProofRole::Server).expect("proof");
        let accepted = AuthMessage::AuthOk { proof };
        socket
            .send(Message::text(accepted.encode()))
            .await
            .expect("auth.ok");
        let mut after_proof = Vec::new();
        while let Some(Ok(message)) = socket.next().await {
            after_proof.push(message);
        }
        after_proof
    });
    (port, task)
}

#[tokio::test]
async fn a_server_that_cannot_prove_itself_gets_no_request() {
    let (port, fake_server) = fake_server_with_token(WRONG_TOKEN).await;

    let outcome = WireClient::authenticate(port, port, TOKEN).await;

    assert!(
        matches!(outcome, Err(HandshakeFailure::Protocol(_))),
        "{:?}",
        outcome.err()
    );
    let after_proof = fake_server.await.expect("fake server");
    assert!(
        after_proof
            .iter()
            .all(|message| matches!(message, Message::Close(_))),
        "the client sent more than a close: {after_proof:?}"
    );
}

#[tokio::test]
async fn a_server_that_knows_the_token_is_accepted() {
    let (port, fake_server) = fake_server_with_token(TOKEN).await;

    let mut client = WireClient::authenticate(port, port, TOKEN)
        .await
        .expect("accepted");
    client.close().await;

    let after_proof = fake_server.await.expect("fake server");
    assert!(
        after_proof
            .iter()
            .all(|message| matches!(message, Message::Close(_)))
    );
}
