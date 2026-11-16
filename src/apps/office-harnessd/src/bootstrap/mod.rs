// SPDX-License-Identifier: AGPL-3.0-or-later

mod access_services;
mod agent_services;
mod database;
mod instance_lock;
mod interrupt_agents;
mod migrations;
mod server_log;
mod services;
mod shutdown_signal;
mod startup_error;
mod storage_alerts;
mod terminal_services;

pub use access_services::{AccessServices, build_access_services};
pub use agent_services::{AgentServices, build_agent_services};
pub use database::open_database;
pub use instance_lock::{InstanceLock, acquire_single_instance};
pub use interrupt_agents::interrupt_running_agents;
pub use migrations::run_migrations;
pub use server_log::{ServerLog, open_log};
pub use services::{Services, start, start_with};
pub use shutdown_signal::wait_for_shutdown_signal;
pub use startup_error::{ALREADY_RUNNING_EXIT_CODE, StartupError};
pub use storage_alerts::{STORAGE_ERROR_EVENT, StorageAlert, StorageAlerts};
pub use terminal_services::{LoginSetup, TerminalServices, build_terminal_services};
