// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_domain::value_objects::Settings;

use crate::common::errors::ApplicationError;
use crate::common::interfaces::SettingsStore;

/// The office settings (`settings.get`).
#[derive(Clone)]
pub struct GetSettingsHandler {
    settings: Arc<dyn SettingsStore>,
}

impl GetSettingsHandler {
    #[must_use]
    pub fn new(settings: Arc<dyn SettingsStore>) -> Self {
        Self { settings }
    }

    /// # Errors
    /// `Storage`.
    pub fn handle(&self) -> Result<Settings, ApplicationError> {
        Ok(self.settings.load()?)
    }
}
