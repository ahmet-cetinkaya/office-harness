// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::value_objects::Settings;

use crate::common::errors::PortError;

pub trait SettingsStore: Send + Sync {
    /// # Errors
    /// `Storage`.
    fn load(&self) -> Result<Settings, PortError>;

    /// # Errors
    /// `Storage`.
    fn save(&self, settings: &Settings) -> Result<(), PortError>;
}
