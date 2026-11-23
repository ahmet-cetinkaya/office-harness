// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::SettingsStore;
use agent_domain::value_objects::{SettingKey, Settings};

use super::*;

/// Settings in memory that can start failing, like a broken disk.
#[derive(Default)]
struct SwitchableSettings {
    settings: Mutex<Settings>,
    failing: Mutex<bool>,
}

impl SettingsStore for SwitchableSettings {
    fn load(&self) -> Result<Settings, PortError> {
        if *self.failing.lock().unwrap_or_else(PoisonError::into_inner) {
            return Err(PortError::Storage {
                message: "the settings cannot be read".to_owned(),
                is_disk_failure: true,
            });
        }
        Ok(self
            .settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone())
    }

    fn save(&self, settings: &Settings) -> Result<(), PortError> {
        self.settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone_from(settings);
        Ok(())
    }
}

fn source_over(store: &Arc<SwitchableSettings>) -> SettingsRateLimits {
    let folder = tempfile::tempdir().expect("folder");
    let config = crate::config::Config {
        data_folder: folder.path().join("office"),
    };
    let log = crate::bootstrap::open_log(&config).expect("log");
    SettingsRateLimits::new(
        GetSettingsHandler::new(Arc::clone(store) as Arc<dyn SettingsStore>),
        log,
    )
}

#[test]
fn the_default_settings_give_the_default_limits() {
    let store = Arc::new(SwitchableSettings::default());

    assert_eq!(source_over(&store).limits(), RateLimits::default());
}

#[test]
fn a_changed_setting_applies_to_the_next_call() {
    let store = Arc::new(SwitchableSettings::default());
    let source = source_over(&store);
    let mut changed = Settings::default();
    changed
        .set_from_text(SettingKey::LimitDirectivesPerAgent, "7")
        .expect("valid");
    changed
        .set_from_text(SettingKey::LimitDirectivesPerTarget, "2")
        .expect("valid");
    changed
        .set_from_text(SettingKey::LimitCreatesPerAgent, "4")
        .expect("valid");
    changed
        .set_from_text(SettingKey::LimitLifecyclePerAgent, "9")
        .expect("valid");
    changed
        .set_from_text(SettingKey::LimitReportsPerAgent, "3")
        .expect("valid");
    store.save(&changed).expect("saved");

    assert_eq!(
        source.limits(),
        RateLimits {
            directive: 7,
            directive_target: 2,
            create: 4,
            lifecycle: 9,
            report: 3,
        }
    );
}

#[test]
fn a_failed_read_falls_back_to_the_default_limits() {
    let store = Arc::new(SwitchableSettings::default());
    let source = source_over(&store);
    *store.failing.lock().expect("lock") = true;

    assert_eq!(source.limits(), RateLimits::default());
}
