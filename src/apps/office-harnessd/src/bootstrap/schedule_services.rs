// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::SettingsStore;
use agent_infrastructure::persistence::SharedConnection;
use schedule_application::common::interfaces::{ScheduleCapSource, ScheduleStore};
use schedule_application::services::RunBook;
use schedule_infrastructure::persistence::ScheduleRepository;

use super::ServerLog;
use crate::bridges::SettingsScheduleCap;

/// The schedule module's store and the cap source over the office settings.
pub struct ScheduleServices {
    pub store: Arc<dyn ScheduleStore>,
    pub cap: Arc<dyn ScheduleCapSource>,
}

/// Builds the schedule services over the shared connection. A run that has no
/// result is a run that a stop cut short: it fails with the reason that the
/// server stopped, and nothing is started for it (0025 D4, PB-1).
#[must_use]
pub fn build_schedule_services(
    connection: &SharedConnection,
    settings: Arc<dyn SettingsStore>,
    log: &ServerLog,
) -> ScheduleServices {
    let store: Arc<dyn ScheduleStore> = Arc::new(ScheduleRepository::new(connection.handle()));
    match RunBook::new(Arc::clone(&store)).fail_open_runs() {
        Ok(0) => {}
        Ok(failed) => log.info(&format!("schedule runs failed at start: {failed}")),
        Err(error) => log.error(&format!("the open schedule runs were not failed: {error}")),
    }
    ScheduleServices {
        store,
        cap: Arc::new(SettingsScheduleCap::new(settings, log.clone())),
    }
}
