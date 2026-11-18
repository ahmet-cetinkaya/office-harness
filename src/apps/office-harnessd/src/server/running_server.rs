// SPDX-License-Identifier: AGPL-3.0-or-later

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use access_application::common::interfaces::NonceSource;
use access_infrastructure::services::RandomNonceSource;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};

use super::agent_requests::AgentGateway;
use super::dispatcher::Dispatcher;
use super::hierarchy_requests::HierarchyGateway;
use super::notification_requests::NotificationGateway;
use super::restart_signal::RestartSignal;
use super::router::build_router;
use super::server_error::ServerError;
use super::server_file::write_server_file;
use super::server_options::ServerOptions;
use super::server_state::ServerState;
use super::terminal_requests::TerminalGateway;
use crate::bootstrap::{ServerLog, Services};
use crate::bridges::{AgentLineBridge, AgentStatusBridge, NotificationBridge, SystemClock};
use crate::config::Config;

/// The loopback WebSocket server, bound and announced in `server.json` but
/// not yet serving.
pub struct Server {
    listener: TcpListener,
    address: SocketAddr,
    state: ServerState,
    shutdown: watch::Sender<bool>,
    announcement_path: PathBuf,
}

impl Server {
    /// Startup steps 7 and 8: binds `127.0.0.1` on a free port and writes
    /// `server.json`. The server never binds another interface.
    ///
    /// # Errors
    /// The bind, the start nonce, or the file write failed.
    pub async fn bind(
        config: &Config,
        services: &Services,
        log: &ServerLog,
        options: ServerOptions,
    ) -> Result<Self, ServerError> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(ServerError::Bind)?;
        let address = listener.local_addr().map_err(ServerError::Bind)?;
        let port = address.port();
        let start_nonce = RandomNonceSource
            .next_nonce()
            .map_err(ServerError::StartNonce)?;
        let announcement_path = config.server_file_path();
        write_server_file(&announcement_path, port, start_nonce.as_str())
            .map_err(ServerError::ServerFile)?;

        let events = services.events.clone();
        events.forward_storage_alerts(services.storage_alerts.subscribe(), log.clone());
        AgentStatusBridge {
            change_status: services.agent.change_status.clone(),
            end_session: services.agent.end_session.clone(),
            runtime: Arc::clone(&services.terminal.runtime),
            hub: events.clone(),
            order: Arc::clone(&services.agent.status_order),
            alerts: services.storage_alerts.clone(),
            log: log.clone(),
            calls: services.access.agent_calls.clone(),
        }
        .spawn(services.terminal.events.subscribe());
        AgentLineBridge::new(services.terminal.events.clone(), events.clone())
            .spawn(services.terminal.events.subscribe_lines());
        NotificationBridge::new(
            services.notification.create.clone(),
            services.agent_bridge_store.clone(),
            Arc::clone(&services.terminal.runtime),
            Arc::new(SystemClock),
            events.clone(),
            services.storage_alerts.clone(),
            log.clone(),
        )
        .spawn(
            services.events.subscribe(),
            services.terminal.events.subscribe(),
        );
        let agents = AgentGateway::new(
            Arc::clone(&services.agent.handlers),
            services.agent.audit.clone(),
            events.clone(),
            services.storage_alerts.clone(),
            Arc::clone(&services.agent.status_order),
            Arc::clone(&services.agent.stopping),
            services.terminal.events.clone(),
            Arc::clone(&services.terminal.runtime),
        );
        let hierarchy =
            HierarchyGateway::new(Arc::clone(&services.hierarchy.handlers), events.clone());
        let notification = NotificationGateway::new(
            Arc::new(services.notification.handlers.clone()),
            events.clone(),
        );
        let (shutdown, stopping) = watch::channel(false);
        let state = ServerState {
            port,
            options,
            authentication: services.access.authentication.clone(),
            agent_authentication: services.access.agent_authentication.clone(),
            dispatcher: Arc::new(Dispatcher::new(services.access.handlers.clone())),
            terminals: Arc::new(TerminalGateway::new(
                services.terminal.handlers.clone(),
                services.agent.saved_outputs.clone(),
            )),
            agents,
            hierarchy,
            send_directive: Arc::clone(&services.hierarchy.send_directive),
            hierarchy_tree: Arc::clone(&services.hierarchy.tree),
            directive_journal: Arc::clone(&services.hierarchy.directive_journal),
            notification,
            events,
            restart: RestartSignal::default(),
            token_generations: services.access.token_store.subscribe(),
            shutdown: stopping,
            handshake_permits: Arc::new(Semaphore::new(options.max_concurrent_handshakes)),
            log: log.clone(),
        };
        Ok(Self {
            listener,
            address,
            state,
            shutdown,
            announcement_path,
        })
    }

    #[must_use]
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        self.address.port()
    }

    /// Completes when a client sent `server.restart`. Take it before `serve`.
    #[must_use]
    pub fn restart_signal(&self) -> RestartSignal {
        self.state.restart.clone()
    }

    /// The routes of this server, for tests that call them without a socket.
    pub fn router(&self) -> Router {
        build_router(self.state.clone())
    }

    /// Serves until `stop` completes, then closes every connection with code
    /// 1001, removes `server.json`, and returns.
    ///
    /// # Errors
    /// `Serve` when the listener fails.
    pub async fn serve(
        self,
        stop: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), ServerError> {
        let Self {
            listener,
            state,
            shutdown,
            announcement_path,
            ..
        } = self;
        let app = build_router(state).into_make_service_with_connect_info::<SocketAddr>();
        let stopping = async move {
            stop.await;
            shutdown.send_replace(true);
        };
        let outcome = axum::serve(listener, app)
            .with_graceful_shutdown(stopping)
            .await;
        // A stale file would point clients at a port that nobody serves.
        let _ = std::fs::remove_file(&announcement_path);
        outcome.map_err(ServerError::Serve)
    }
}
