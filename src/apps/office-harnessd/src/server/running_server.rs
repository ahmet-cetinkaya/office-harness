// SPDX-License-Identifier: AGPL-3.0-or-later

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use access_application::common::interfaces::NonceSource;
use access_infrastructure::services::RandomNonceSource;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};

use super::agent_requests::AgentGateway;
use super::dispatcher::Dispatcher;
use super::event_hub::EventHub;
use super::hierarchy_requests::HierarchyGateway;
use super::hold_requests::HoldGateway;
use super::idle_sleep_run::IdleSleeper;
use super::notification_requests::NotificationGateway;
use super::restart_signal::RestartSignal;
use super::resume_requests::ResumeGateway;
use super::resume_run::ResumeContext;
use super::router::build_router;
use super::server_error::ServerError;
use super::server_file::write_server_file;
use super::server_options::ServerOptions;
use super::server_state::ServerState;
use super::template_requests::TemplateGateway;
use super::terminal_requests::TerminalGateway;
use crate::bootstrap::{ServerLog, Services};
use crate::bridges::{AgentLineBridge, AgentStatusBridge};
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

/// The bulk resume of 0013: it starts through the agent gateway, so every
/// start keeps the status order and the action record.
fn resume_gateway(
    services: &Services,
    agents: &AgentGateway,
    events: &EventHub,
    check_period: Duration,
) -> ResumeGateway {
    ResumeGateway::new(
        ResumeContext {
            agents: agents.clone(),
            store: Arc::clone(&services.agent_bridge_store),
            holds: Arc::clone(&services.hierarchy.holds),
            events: events.clone(),
            alerts: services.storage_alerts.clone(),
            check_period,
        },
        Arc::clone(&services.hierarchy.tree),
    )
}

/// The idle sleep task of 0018: it sleeps through the agent gateway, so
/// every sleep keeps the status order and the action record.
fn idle_sleeper(
    services: &Services,
    agents: &AgentGateway,
    log: &ServerLog,
    options: &ServerOptions,
) -> IdleSleeper {
    IdleSleeper::new(
        Arc::clone(&services.agent.sleep_gate),
        Arc::clone(&services.agent.settings),
        Arc::clone(&services.agent_bridge_store),
        agents.clone(),
        options.idle_sleep_tick,
        options.idle_sleep_minute,
        log.clone(),
    )
}

/// The terminal requests of a user client, over the terminal services.
fn terminal_gateway(services: &Services) -> Arc<TerminalGateway> {
    Arc::new(TerminalGateway::new(
        services.terminal.handlers.clone(),
        services.agent.saved_outputs.clone(),
        services.terminal.person_input.clone(),
    ))
}

/// The user's `notification.*` and `needs_you.*` requests.
fn notification_gateway(services: &Services) -> NotificationGateway {
    NotificationGateway::new(
        Arc::new(services.notification.handlers.clone()),
        services.needs_you_bridge.clone(),
    )
}

/// The user's `template.*` requests, over the template services.
fn template_gateway(services: &Services) -> TemplateGateway {
    TemplateGateway::new(
        Arc::clone(&services.template.handlers),
        Arc::clone(&services.template.create),
        services.events.clone(),
        services.storage_alerts.clone(),
    )
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
            sign_in_ends: services.terminal.sign_in_ends.clone(),
            journal: services.agent.journal.clone(),
        }
        .spawn(services.terminal.events.subscribe());
        AgentLineBridge::new(services.terminal.events.clone(), events.clone())
            .spawn(services.terminal.events.subscribe_lines());
        services.notification_bridge.clone().spawn(
            services.events.subscribe(),
            services.terminal.events.subscribe(),
        );
        let holds = HoldGateway::new(
            Arc::clone(&services.hierarchy.holds),
            Arc::clone(&services.agent_bridge_store),
            services.agent.office_actions.clone(),
            events.clone(),
            services.storage_alerts.clone(),
        );
        let agents = AgentGateway::new(
            Arc::clone(&services.agent.handlers),
            services.agent.audit.clone(),
            events.clone(),
            services.storage_alerts.clone(),
            Arc::clone(&services.agent.status_order),
            Arc::clone(&services.agent.task_order),
            Arc::clone(&services.agent.stopping),
            services.terminal.events.clone(),
            Arc::clone(&services.terminal.runtime),
            Arc::clone(&services.hierarchy.tree),
            holds.clone(),
            services.needs_you_bridge.clone(),
        )
        .with_sign_in_checks(services.agent.sign_in_checks.clone());
        let hierarchy = HierarchyGateway::new(
            Arc::clone(&services.hierarchy.handlers),
            services.agent.agent_reader.clone(),
            events.clone(),
            services.storage_alerts.clone(),
        );
        let resume = resume_gateway(services, &agents, &events, options.resume_check_period);
        let (shutdown, stopping) = watch::channel(false);
        idle_sleeper(services, &agents, log, &options).spawn(stopping.clone());
        let state = ServerState {
            port,
            options,
            authentication: services.access.authentication.clone(),
            agent_authentication: services.access.agent_authentication.clone(),
            dispatcher: Arc::new(Dispatcher::new(services.access.handlers.clone())),
            terminals: terminal_gateway(services),
            agents,
            hierarchy,
            holds,
            hold_service: Arc::clone(&services.hierarchy.holds),
            send_directive: Arc::clone(&services.hierarchy.send_directive),
            send_report: Arc::clone(&services.hierarchy.send_report),
            hierarchy_tree: Arc::clone(&services.hierarchy.tree),
            directive_journal: Arc::clone(&services.hierarchy.directive_journal),
            notification: notification_gateway(services),
            templates: template_gateway(services),
            resume,
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
