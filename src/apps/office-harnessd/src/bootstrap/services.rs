// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Machine;
use agent_infrastructure::persistence::{
    AgentRepository, LEDGER_TABLE, MachineRepository, ProjectRepository, SCRIPTS,
    SettingsRepository, SharedConnection,
};
use gethostname::gethostname;

use super::{
    AccessServices, ServerLog, StartupError, StorageAlerts, TerminalServices,
    build_access_services, build_terminal_services, open_database, run_migrations,
};
use crate::config::Config;

const FALLBACK_HOST_NAME: &str = "localhost";

/// The adapters and use-case services that request handlers will use.
pub struct Services {
    pub machine: Machine,
    pub agents: AgentRepository,
    pub projects: ProjectRepository,
    pub settings: SettingsRepository,
    pub storage_alerts: StorageAlerts,
    pub access: AccessServices,
    pub terminal: TerminalServices,
}

/// Startup steps 3 to 6: run each module's migrations, mark the agents of an
/// unclean stop as `interrupted` (FR-004), and build the adapters, the office
/// token and the terminal runtime included.
///
/// # Errors
/// Any database, migration, or persistence failure.
pub fn start(config: &Config, log: &ServerLog) -> Result<Services, StartupError> {
    let mut connection = open_database(&config.database_path())?;
    run_migrations(&mut connection, LEDGER_TABLE, SCRIPTS)?;

    let connection = SharedConnection::new(connection);
    let agents = AgentRepository::new(connection.clone());
    let interrupted = agents.mark_running_as_interrupted()?;
    log.info(&format!(
        "agents marked interrupted at start: {interrupted}"
    ));

    let machine = MachineRepository::new(connection.clone()).ensure_local(&host_name())?;
    let settings = SettingsRepository::new(connection.clone());
    let terminal = build_terminal_services(config, &settings.load()?)?;
    Ok(Services {
        machine,
        agents,
        projects: ProjectRepository::new(connection.clone()),
        settings,
        storage_alerts: StorageAlerts::new(log.clone()),
        access: build_access_services(config)?,
        terminal,
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
