// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::SettingsStore;
use schedule_application::common::interfaces::ScheduleCapSource;
use schedule_domain::ScheduleCap;

use crate::bootstrap::ServerLog;

/// The schedule cap as the office settings hold it, read at each save so a
/// changed cap applies to the next save (0025/FR-002).
pub struct SettingsScheduleCap {
    settings: Arc<dyn SettingsStore>,
    log: ServerLog,
}

impl SettingsScheduleCap {
    #[must_use]
    pub fn new(settings: Arc<dyn SettingsStore>, log: ServerLog) -> Self {
        Self { settings, log }
    }
}

impl ScheduleCapSource for SettingsScheduleCap {
    fn cap(&self) -> ScheduleCap {
        match self.settings.load() {
            Ok(settings) => ScheduleCap::new(i64::from(settings.schedule_cap)).unwrap_or_default(),
            Err(error) => {
                self.log.error(&format!(
                    "the schedule cap cannot be read, the default applies: {error}"
                ));
                ScheduleCap::default()
            }
        }
    }
}
