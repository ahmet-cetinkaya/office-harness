// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use access_application::commands::authenticate_agent_call::AgentAuthenticationHandler;
use access_application::commands::authenticate_connection::AuthenticationHandler;
use hierarchy_application::commands::hold::HoldService;
use hierarchy_application::commands::send_directive::SendDirectiveHandler;
use hierarchy_application::commands::send_report::SendReportHandler;
use hierarchy_application::queries::tree::TreeQuery;
use kernel::TextRedactor;
use tokio::sync::{Semaphore, watch};

use crate::bridges::DirectiveJournalBridge;

use super::agent_requests::AgentGateway;
use super::dispatcher::Dispatcher;
use super::event_hub::EventHub;
use super::hierarchy_requests::HierarchyGateway;
use super::hold_requests::HoldGateway;
use super::hook_requests::HookGateway;
use super::notification_requests::NotificationGateway;
use super::restart_signal::RestartSignal;
use super::resume_requests::ResumeGateway;
use super::secret_requests::SecretGateway;
use super::server_options::ServerOptions;
use super::temp_requests::TempGateway;
use super::template_requests::TemplateGateway;
use super::terminal_requests::TerminalGateway;
use crate::bootstrap::ServerLog;

/// What every connection needs; cloned for each of them.
#[derive(Clone)]
pub struct ServerState {
    pub port: u16,
    pub options: ServerOptions,
    pub authentication: AuthenticationHandler,
    pub agent_authentication: AgentAuthenticationHandler,
    pub dispatcher: Arc<Dispatcher>,
    pub terminals: Arc<TerminalGateway>,
    pub agents: AgentGateway,
    pub hierarchy: HierarchyGateway,
    /// The user's `hold.*` and `office.actions` requests. Only the user
    /// connection routes to it; the agent connection never does (0007).
    pub holds: HoldGateway,
    /// The gate the agent-call gateway asks before an office request runs.
    pub hold_service: Arc<HoldService>,
    /// Sends one directive from this agent to its member (FR-009), answered
    /// on the agent connections.
    pub send_directive: Arc<SendDirectiveHandler>,
    /// Sends `report.send` upward (0015).
    pub send_report: Arc<SendReportHandler>,
    /// The tree the agent-call gateway reads for the descendant rule
    /// (FR-008): a request an agent sends names an agent in its own subtree,
    /// or it never reaches a use case.
    pub hierarchy_tree: Arc<TreeQuery>,
    /// Journals the refusals the agent-call gateway itself rejects; the use
    /// case journals the attempts it runs.
    pub directive_journal: Arc<DirectiveJournalBridge>,
    /// The status-hook reports of the `claude` sessions (0024). The route
    /// answers with the session secret, never with the office token.
    pub hooks: HookGateway,
    pub notification: NotificationGateway,
    /// The user's `secret.*` requests and the agent's `secret.available` and
    /// `secret.get` (0026). Only the agent connection routes the second pair.
    pub secrets: SecretGateway,
    /// Replaces secret values in the text an agent sends, before any use case
    /// stores or passes it on (0026/FR-012).
    pub redactor: Arc<dyn TextRedactor>,
    /// The user's `template.*` requests. Only the user connection routes to
    /// it; the agent connection never does (0009).
    pub templates: TemplateGateway,
    /// The user's `resume.*` requests (0013). Only the user connection routes
    /// to it; the agent connection never does (A10).
    pub resume: ResumeGateway,
    /// The user's `temp.*` requests, and the ender that the done report of a
    /// temp calls (0020). The agent connection never routes requests to it.
    pub temps: TempGateway,
    pub events: EventHub,
    /// Raised by `server.restart`.
    pub restart: RestartSignal,
    /// The latest token generation. A connection that authenticated with an
    /// older one closes.
    pub token_generations: watch::Receiver<u64>,
    /// Turns true when the server stops.
    pub shutdown: watch::Receiver<bool>,
    /// One permit per connection that may be in the handshake.
    pub handshake_permits: Arc<Semaphore>,
    pub log: ServerLog,
}
