// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::queries::get_settings::GetSettingsHandler;

/// The quiet period the terminal waits before it calls an agent "waiting for
/// input", read when the event arrives so a `settings.set` applies at once.
pub trait QuietPeriodSource: Send + Sync {
    /// # Errors
    /// The text says why the setting could not be read.
    fn quiet_period_ms(&self) -> Result<u64, String>;
}

/// Bridges the port to the agent module's settings query.
pub struct SettingsQuietPeriod {
    settings: GetSettingsHandler,
}

impl SettingsQuietPeriod {
    #[must_use]
    pub fn new(settings: GetSettingsHandler) -> Self {
        Self { settings }
    }
}

impl QuietPeriodSource for SettingsQuietPeriod {
    fn quiet_period_ms(&self) -> Result<u64, String> {
        self.settings
            .handle()
            .map(|settings| settings.quiet_period_ms)
            .map_err(|error| error.to_string())
    }
}
