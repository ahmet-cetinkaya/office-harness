// SPDX-License-Identifier: AGPL-3.0-or-later

use std::future::Future;

use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use wire::{AuthMessage, ProofRole, compute_proof};

use super::*;

const TOKEN: &str = "the-office-token";
const NONCE: &str = "ab12ab12ab12ab12ab12ab12ab12ab12ab12ab12ab12ab12ab12ab12ab12ab12";

type ServerSocket = WebSocketStream<TcpStream>;

/// Runs one fake server connection and returns the port it listens on.
async fn fake_server<Script, Run>(script: Script) -> u16
where
    Script: FnOnce(ServerSocket, u16) -> Run + Send + 'static,
    Run: Future<Output = ()> + Send,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("address").port();
    tokio::spawn(async move {
        let (stream, _peer) = listener.accept().await.expect("accept");
        let socket = accept_async(stream).await.expect("upgrade");
        script(socket, port).await;
    });
    port
}

async fn send_challenge(socket: &mut ServerSocket) {
    let challenge = AuthMessage::Challenge {
        nonce: NONCE.to_owned(),
    };
    socket
        .send(Message::text(challenge.encode()))
        .await
        .expect("challenge");
}

async fn receive_client_proof(socket: &mut ServerSocket) -> String {
    let Some(Ok(Message::Text(frame))) = socket.next().await else {
        panic!("the client sent no text frame");
    };
    match AuthMessage::decode(frame.as_str()) {
        Some(AuthMessage::Auth { proof, .. }) => proof,
        other => panic!("expected an auth frame, got {other:?}"),
    }
}

async fn close_unauthorized(socket: &mut ServerSocket) {
    close_with(socket, CLOSE_UNAUTHORIZED).await;
}

async fn close_with(socket: &mut ServerSocket, code: u16) {
    let frame = CloseFrame {
        code: CloseCode::from(code),
        reason: "".into(),
    };
    socket.close(Some(frame)).await.expect("close");
}

fn server_proof(token: &str, port: u16) -> String {
    compute_proof(token, NONCE, port, ProofRole::Server).expect("proof")
}

#[tokio::test]
async fn a_server_that_proves_itself_gives_a_socket() {
    let port = fake_server(|mut socket, port| async move {
        send_challenge(&mut socket).await;
        let client_proof = receive_client_proof(&mut socket).await;
        let expected = compute_proof(TOKEN, NONCE, port, ProofRole::Client).expect("proof");
        assert_eq!(client_proof, expected);
        let accepted = AuthMessage::AuthOk {
            proof: server_proof(TOKEN, port),
        };
        socket
            .send(Message::text(accepted.encode()))
            .await
            .expect("auth.ok");
    })
    .await;

    assert!(open_authenticated(port, TOKEN).await.is_ok());
}

#[tokio::test]
async fn a_server_with_a_wrong_proof_is_not_reachable() {
    let port = fake_server(|mut socket, port| async move {
        send_challenge(&mut socket).await;
        receive_client_proof(&mut socket).await;
        let accepted = AuthMessage::AuthOk {
            proof: server_proof("another-token", port),
        };
        socket
            .send(Message::text(accepted.encode()))
            .await
            .expect("auth.ok");
    })
    .await;

    assert!(matches!(
        open_authenticated(port, TOKEN).await,
        Err(Failure::NotReachable)
    ));
}

#[tokio::test]
async fn a_close_with_the_unauthorized_code_after_the_proof_is_a_rejection() {
    let port = fake_server(|mut socket, _port| async move {
        send_challenge(&mut socket).await;
        receive_client_proof(&mut socket).await;
        close_unauthorized(&mut socket).await;
    })
    .await;

    assert!(matches!(
        open_authenticated(port, TOKEN).await,
        Err(Failure::Rejected)
    ));
}

#[tokio::test]
async fn an_unauthorized_close_before_the_challenge_is_not_reachable() {
    let port = fake_server(|mut socket, _port| async move {
        close_unauthorized(&mut socket).await;
    })
    .await;

    assert!(matches!(
        open_authenticated(port, TOKEN).await,
        Err(Failure::NotReachable)
    ));
}

#[tokio::test]
async fn a_close_because_every_handshake_slot_is_busy_is_not_a_rejection() {
    let port = fake_server(|mut socket, _port| async move {
        close_with(&mut socket, CLOSE_TRY_AGAIN_LATER).await;
    })
    .await;

    assert!(matches!(
        open_authenticated(port, TOKEN).await,
        Err(Failure::Busy)
    ));
}

#[tokio::test]
async fn nothing_listening_is_not_reachable() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("address").port();
    drop(listener);

    assert!(matches!(
        open_authenticated(port, TOKEN).await,
        Err(Failure::NotReachable)
    ));
}
