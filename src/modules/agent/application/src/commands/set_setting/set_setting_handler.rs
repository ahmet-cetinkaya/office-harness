// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::value_objects::{SettingKey, Settings};

use crate::common::errors::ApplicationError;
use crate::common::interfaces::SettingsStore;

/// Changes one setting (`settings.set`).
#[derive(Clone)]
pub struct SetSettingHandler {
    settings: Arc<dyn SettingsStore>,
}

impl SetSettingHandler {
    #[must_use]
    pub fn new(settings: Arc<dyn SettingsStore>) -> Self {
        Self { settings }
    }

    /// Returns all settings after the change.
    ///
    /// # Errors
    /// `Invalid`, naming the setting, for an unknown key or a bad value; the
    /// stored settings stay as they were.
    pub fn handle(&self, key: &str, value: &str) -> Result<Settings, ApplicationError> {
        let key = SettingKey::parse(key)?;
        let mut settings = self.settings.load()?;
        settings.set_from_text(key, value)?;
        self.settings.save(&settings)?;
        Ok(settings)
    }
}

#[cfg(test)]
#[path = "set_setting_handler.tests.rs"]
mod tests;
