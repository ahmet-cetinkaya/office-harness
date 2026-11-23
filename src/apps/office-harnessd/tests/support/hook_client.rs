// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! Posts status-hook reports to a running test server the way the CLI's hooks
//! do (0024): one request, the session secret in the `Authorization` header.

use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use access_application::common::interfaces::HookSecretStore;
use kernel::AgentId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::office_client::Office;

/// The session id that a test gives its agent's hook secret.
pub const HOOK_SESSION: &str = "00000000-0000-0000-0000-00000000aaaa";

/// A session that can post reports for one agent of a test office.
pub struct HookPoster {
    port: u16,
    secret: String,
}

impl HookPoster {
    /// Issues a secret for `agent_id`'s running session, as the launch of a
    /// `claude` session does.
    pub fn for_agent(office: &Office, agent_id: &str) -> Self {
        let services = &office.serving.office.services;
        let agent = AgentId::from_str(agent_id).expect("agent id");
        let secret = services
            .access
            .hook_secrets
            .issue(agent, HOOK_SESSION)
            .expect("issue");
        Self {
            port: office.serving.office.port,
            secret: secret.expose_secret().to_owned(),
        }
    }

    /// Posts one report with the CLI's event name and extra JSON fields (a
    /// string such as `"agent_id":"helper"`, or empty). Returns the status
    /// code.
    pub async fn report(&self, cli_event: &str, extra_fields: &str) -> u16 {
        let extra = if extra_fields.is_empty() {
            String::new()
        } else {
            format!(",{extra_fields}")
        };
        let body = format!(r#"{{"hook_event_name":"{cli_event}"{extra}}}"#);
        let request = format!(
            "POST /v1/hook/{HOOK_SESSION} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\
             Content-Type: application/json\r\nAuthorization: Bearer {secret}\r\n\
             Content-Length: {length}\r\nConnection: close\r\n\r\n{body}",
            port = self.port,
            secret = self.secret,
            length = body.len(),
        );
        let address = SocketAddr::from(([127, 0, 0, 1], self.port));
        let mut stream = TcpStream::connect(address).await.expect("connect");
        stream.write_all(request.as_bytes()).await.expect("request");
        let mut answer = Vec::new();
        let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut answer)).await;
        String::from_utf8_lossy(&answer)
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("status line")
    }
}
