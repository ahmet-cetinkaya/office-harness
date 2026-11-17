// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::Clock;
use agent_domain::entities::Machine;
use agent_infrastructure::persistence::{
    AgentRepository, LEDGER_TABLE, MachineRepository, ProjectRepository, SCRIPTS,
    SessionRepository, SettingsRepository, SharedConnection,
};
use audit_infrastructure::persistence::{
    LEDGER_TABLE as AUDIT_LEDGER_TABLE, SCRIPTS as AUDIT_SCRIPTS,
};
use gethostname::gethostname;

use super::{
    AccessServices, AgentServices, HierarchyServices, LoginSetup, ServerLog, StartupError,
    StorageAlerts, TerminalServices, build_access_services, build_agent_services,
    build_hierarchy_services, build_terminal_services, open_database, run_migrations,
};
use crate::bridges::SystemClock;
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
    run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS)?;
    run_migrations(&mut connection, AUDIT_LEDGER_TABLE, AUDIT_SCRIPTS)?;

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
    let agent = build_agent_services(
        &connection,
        machine.id,
        &terminal,
        &storage_alerts,
        attribution,
        log,
    );
    let hierarchy = build_hierarchy_services(&connection, &agent, &storage_alerts, log)?;
    Ok(Services {
        machine,
        agents,
        projects: ProjectRepository::new(connection),
        settings,
        storage_alerts,
        events,
        access: build_access_services(config)?,
        terminal,
        agent,
        hierarchy,
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
