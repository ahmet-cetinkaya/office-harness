// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

use std::collections::VecDeque;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use wire::{ClientHandshake, HandshakeError};

const WAIT: Duration = Duration::from_secs(10);

/// What the next WebSocket message was.
#[derive(Debug, PartialEq, Eq)]
pub enum Received {
    Text(String),
    Closed(Option<u16>),
    Other,
}

/// A test client for the office protocol, built on the `wire` handshake.
pub struct WireClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    /// Binary frames that arrived while the test waited for a text answer.
    queued_binary: VecDeque<Vec<u8>>,
}

impl WireClient {
    pub async fn connect(port: u16) -> Result<Self, Error> {
        Self::connect_with_headers(port, &[]).await
    }

    /// Connects with the given headers replacing the ones the client would
    /// send by itself.
    pub async fn connect_with_headers(
        port: u16,
        headers: &[(&'static str, &str)],
    ) -> Result<Self, Error> {
        let mut request = format!("ws://127.0.0.1:{port}/v1/ws").into_client_request()?;
        for (name, value) in headers {
            request
                .headers_mut()
                .insert(*name, HeaderValue::from_str(value).expect("header value"));
        }
        let (stream, _) = connect_async(request).await?;
        Ok(Self {
            stream,
            queued_binary: VecDeque::new(),
        })
    }

    /// The whole handshake: it answers the challenge for `claimed_port`, and
    /// checks the server proof. When the server cannot prove itself the
    /// client closes at once and sends nothing else.
    pub async fn authenticate(
        port: u16,
        claimed_port: u16,
        token: &str,
    ) -> Result<Self, HandshakeFailure> {
        let mut client = Self::connect(port)
            .await
            .map_err(|_| HandshakeFailure::Connect)?;
        let mut handshake = ClientHandshake::new(token, claimed_port);
        let challenge = match client.receive().await {
            Received::Text(text) => text,
            Received::Closed(code) => return Err(HandshakeFailure::Refused(code)),
            Received::Other => return Err(HandshakeFailure::NoChallengeFrame),
        };
        let answer = handshake
            .answer_challenge(&challenge)
            .map_err(HandshakeFailure::Protocol)?;
        client.send_text(&answer).await;
        let accepted = match client.receive().await {
            Received::Text(text) => text,
            Received::Closed(code) => return Err(HandshakeFailure::Refused(code)),
            Received::Other => return Err(HandshakeFailure::NoChallengeFrame),
        };
        if let Err(error) = handshake.confirm_server(&accepted) {
            client.close().await;
            return Err(HandshakeFailure::Protocol(error));
        }
        Ok(client)
    }

    pub async fn authenticate_ok(port: u16, token: &str) -> Self {
        Self::authenticate(port, port, token)
            .await
            .expect("authenticated")
    }

    pub async fn send_text(&mut self, text: &str) {
        // A send to a connection the server already closed fails; the test
        // reads the close code next.
        let _ = self.stream.send(Message::text(text.to_owned())).await;
    }

    pub async fn send_binary(&mut self, bytes: Vec<u8>) {
        let _ = self.stream.send(Message::binary(bytes)).await;
    }

    pub async fn close(&mut self) {
        let _ = self.stream.send(Message::Close(None)).await;
    }

    pub async fn receive(&mut self) -> Received {
        match timeout(WAIT, self.stream.next())
            .await
            .expect("the server answers in time")
        {
            Some(Ok(Message::Text(text))) => Received::Text(text.as_str().to_owned()),
            Some(Ok(Message::Close(frame))) => {
                Received::Closed(frame.map(|frame| u16::from(frame.code)))
            }
            Some(Ok(_)) => Received::Other,
            Some(Err(_)) | None => Received::Closed(None),
        }
    }

    /// The next binary frame, in the order the server sent them.
    pub async fn receive_binary(&mut self) -> Vec<u8> {
        if let Some(frame) = self.queued_binary.pop_front() {
            return frame;
        }
        match self.next_message().await {
            Some(Ok(Message::Binary(bytes))) => bytes.to_vec(),
            other => panic!("expected a binary frame, got {other:?}"),
        }
    }

    async fn next_message(&mut self) -> Option<Result<Message, Error>> {
        timeout(WAIT, self.stream.next())
            .await
            .expect("the server answers in time")
    }

    /// Sends a request with fields and returns the answer with the same id.
    pub async fn request_with(&mut self, body: Value) -> Value {
        self.send_text(&body.to_string()).await;
        self.receive_json().await
    }

    /// The next text message as JSON. Binary frames that come first are kept
    /// for `receive_binary`.
    pub async fn receive_json(&mut self) -> Value {
        loop {
            match self.next_message().await {
                Some(Ok(Message::Text(text))) => {
                    return serde_json::from_str(text.as_str()).expect("json");
                }
                Some(Ok(Message::Binary(bytes))) => self.queued_binary.push_back(bytes.to_vec()),
                other => panic!("expected a text frame, got {other:?}"),
            }
        }
    }

    /// The next message must be the close frame; returns its code.
    pub async fn expect_close(&mut self) -> Option<u16> {
        match self.receive().await {
            Received::Closed(code) => code,
            other => panic!("expected a close frame, got {other:?}"),
        }
    }

    /// Sends a request and returns the answer with the same id.
    pub async fn request(&mut self, id: u32, kind: &str) -> Value {
        self.send_text(&serde_json::json!({"id": id, "type": kind}).to_string())
            .await;
        self.receive_json().await
    }
}

#[derive(Debug)]
pub enum HandshakeFailure {
    Connect,
    NoChallengeFrame,
    Protocol(HandshakeError),
    /// The server closed instead of sending `auth.ok`.
    Refused(Option<u16>),
}
