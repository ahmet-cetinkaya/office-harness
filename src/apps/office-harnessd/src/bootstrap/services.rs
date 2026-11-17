// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::Clock;
use agent_domain::entities::Machine;
use agent_infrastructure::persistence::{
    AgentRepository, LEDGER_TABLE, MachineRepository, PRE_MIGRATION_CHECKS, ProjectRepository,
    SCRIPTS, SessionRepository, SettingsRepository, SharedConnection,
};
use audit_infrastructure::persistence::{
    LEDGER_TABLE as AUDIT_LEDGER_TABLE, SCRIPTS as AUDIT_SCRIPTS,
};
use gethostname::gethostname;
use notification_infrastructure::persistence::{
    LEDGER_TABLE as NOTIFICATION_LEDGER_TABLE, SCRIPTS as NOTIFICATION_SCRIPTS,
};

use super::{
    AccessServices, AgentServices, HierarchyServices, LoginSetup, NotificationServices, ServerLog,
    StartupError, StorageAlerts, TerminalServices, build_access_services, build_agent_services,
    build_hierarchy_services, build_notification_services, build_terminal_services, open_database,
    run_migrations,
};
use crate::bridges::{AgentCalls, CallHelper, SystemClock};
use crate::config::Config;
use crate::server::EventHub;

const FALLBACK_HOST_NAME: &str = "localhost";

/// The adapters and use-case services that request handlers will use.
pub struct Services {
    pub machine: Machine,
    pub agents: AgentRepository,
    pub projects: ProjectRepository,
    pub settings: SettingsRepository,
    pub storage_alerts: StorageAlerts,
    pub events: EventHub,
    pub access: AccessServices,
    pub terminal: TerminalServices,
    pub agent: AgentServices,
    pub hierarchy: HierarchyServices,
    pub notification: NotificationServices,
    /// The agent store as the bridge reads it, for the agent name a
    /// notification copies at creation.
    pub agent_bridge_store: Arc<dyn agent_application::common::interfaces::AgentStore>,
}

/// Startup steps 3 to 6 with the user's real login shell.
///
/// # Errors
/// Any database, migration, or persistence failure.
pub fn start(config: &Config, log: &ServerLog) -> Result<Services, StartupError> {
    start_with(config, log, LoginSetup::detect())
}

/// Startup steps 3 to 6: run each module's migrations, mark the agents of an
/// unclean stop as `interrupted` (FR-004), and build the adapters, the office
/// token and the terminal runtime included. `login` says how a launched
/// agent gets its environment.
///
/// # Errors
/// Any database, migration, or persistence failure.
pub fn start_with(
    config: &Config,
    log: &ServerLog,
    login: LoginSetup,
) -> Result<Services, StartupError> {
    // Before any agent starts: a stop sends Ctrl+C first, and a server that a
    // parent started in a new process group would pass "ignore Ctrl+C" on to
    // every agent. The forced end after the grace period stays the fallback.
    if let Err(reason) = console_control::enable_interrupt_for_children() {
        log.error(&format!(
            "warning: Ctrl+C stays off for the agents; a stop waits for the grace period: {reason}"
        ));
    }
    let mut connection = open_database(&config.database_path())?;
    run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS, PRE_MIGRATION_CHECKS)?;
    run_migrations(&mut connection, AUDIT_LEDGER_TABLE, AUDIT_SCRIPTS, &[])?;
    run_migrations(
        &mut connection,
        NOTIFICATION_LEDGER_TABLE,
        NOTIFICATION_SCRIPTS,
        &[],
    )?;

    let connection = SharedConnection::new(connection);
    let agents = AgentRepository::new(connection.clone());
    let interrupted = agents.mark_running_as_interrupted()?;
    SessionRepository::new(connection.clone()).mark_running_as_interrupted(SystemClock.now())?;
    log.info(&format!(
        "agents marked interrupted at start: {interrupted}"
    ));

    let machine = MachineRepository::new(connection.clone()).ensure_local(&host_name())?;
    let settings = SettingsRepository::new(connection.clone());
    let attribution = login.attribution;
    let terminal = build_terminal_services(config, &settings.load()?, login)?;
    let events = EventHub::new();
    let storage_alerts = StorageAlerts::new(log.clone());
    let access = build_access_services(config)?;
    let agent_calls = AgentCalls {
        store: access.agent_calls.clone(),
        helper: CallHelper {
            server_binary: std::env::current_exe()
                .map_err(StartupError::ServerPath)?
                .display()
                .to_string(),
            server_file: config.server_file_path().display().to_string(),
        },
    };
    let agent = build_agent_services(
        &connection,
        machine.id,
        &terminal,
        &storage_alerts,
        attribution,
        log,
        agent_calls,
    );
    let hierarchy = build_hierarchy_services(&connection, &agent, &terminal, &storage_alerts, log)?;
    let notification = build_notification_services(connection.handle());
    // The bridge reads the agent name at creation through the agent module's
    // own store adapter; the repository is only a handle on the connection.
    let agent_bridge_store: Arc<dyn agent_application::common::interfaces::AgentStore> =
        Arc::new(AgentRepository::new(connection.clone()));
    Ok(Services {
        machine,
        agents,
        projects: ProjectRepository::new(connection),
        settings,
        storage_alerts,
        events,
        access,
        terminal,
        agent,
        hierarchy,
        notification,
        agent_bridge_store,
    })
}

fn host_name() -> String {
    let name = gethostname().to_string_lossy().trim().to_owned();
    if name.is_empty() {
        FALLBACK_HOST_NAME.to_owned()
    } else {
        name
    }
}
