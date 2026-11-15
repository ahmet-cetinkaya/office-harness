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

use super::dispatcher::Dispatcher;
use super::event_hub::EventHub;
use super::router::build_router;
use super::server_error::ServerError;
use super::server_file::write_server_file;
use super::server_options::ServerOptions;
use super::server_state::ServerState;
use crate::bootstrap::{ServerLog, Services};
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

        let events = EventHub::new();
        events.forward_storage_alerts(services.storage_alerts.subscribe());
        let (shutdown, stopping) = watch::channel(false);
        let state = ServerState {
            port,
            options,
            authentication: services.access.authentication.clone(),
            dispatcher: Arc::new(Dispatcher::new(services.access.handlers.clone())),
            events,
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
