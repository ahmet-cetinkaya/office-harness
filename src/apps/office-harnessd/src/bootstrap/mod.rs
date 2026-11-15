// SPDX-License-Identifier: AGPL-3.0-or-later

mod database;
mod instance_lock;
mod migrations;
mod server_log;
mod services;
mod shutdown_signal;
mod startup_error;
mod storage_alerts;

pub use database::open_database;
pub use instance_lock::{InstanceLock, acquire_single_instance};
pub use migrations::run_migrations;
pub use server_log::{ServerLog, open_log};
pub use services::{Services, start};
pub use shutdown_signal::wait_for_shutdown_signal;
pub use startup_error::StartupError;
pub use storage_alerts::{STORAGE_ERROR_EVENT, StorageAlert, StorageAlerts};
