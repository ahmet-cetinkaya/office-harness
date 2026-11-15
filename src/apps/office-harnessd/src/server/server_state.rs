// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use access_application::commands::authenticate_connection::AuthenticationHandler;
use tokio::sync::{Semaphore, watch};

use super::agent_requests::AgentGateway;
use super::dispatcher::Dispatcher;
use super::event_hub::EventHub;
use super::server_options::ServerOptions;
use super::terminal_requests::TerminalGateway;
use crate::bootstrap::ServerLog;

/// What every connection needs; cloned for each of them.
#[derive(Clone)]
pub struct ServerState {
    pub port: u16,
    pub options: ServerOptions,
    pub authentication: AuthenticationHandler,
    pub dispatcher: Arc<Dispatcher>,
    pub terminals: Arc<TerminalGateway>,
    pub agents: AgentGateway,
    pub events: EventHub,
    /// The latest token generation. A connection that authenticated with an
    /// older one closes.
    pub token_generations: watch::Receiver<u64>,
    /// Turns true when the server stops.
    pub shutdown: watch::Receiver<bool>,
    /// One permit per connection that may be in the handshake.
    pub handshake_permits: Arc<Semaphore>,
    pub log: ServerLog,
}
