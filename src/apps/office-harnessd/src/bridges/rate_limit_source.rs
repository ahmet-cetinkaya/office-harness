// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::queries::get_settings::GetSettingsHandler;
use hierarchy_application::common::ports::RateLimitSource;
use hierarchy_domain::RateLimits;

use crate::bootstrap::ServerLog;

/// Answers the hierarchy's limit port from the agent module's settings. It
/// reads on every call, so a `settings.set` applies to the next agent action
/// (FR-009). A failed read falls back to the default limits and is logged:
/// the gate must keep limiting, and the settings failure itself already
/// reaches the user through the requests that read or write settings.
pub struct SettingsRateLimits {
    settings: GetSettingsHandler,
    log: ServerLog,
}

impl SettingsRateLimits {
    #[must_use]
    pub fn new(settings: GetSettingsHandler, log: ServerLog) -> Self {
        Self { settings, log }
    }
}

impl RateLimitSource for SettingsRateLimits {
    fn limits(&self) -> RateLimits {
        match self.settings.handle() {
            Ok(settings) => RateLimits {
                directive: settings.limit_directives_per_agent,
                directive_target: settings.limit_directives_per_target,
                create: settings.limit_creates_per_agent,
                lifecycle: settings.limit_lifecycle_per_agent,
            },
            Err(error) => {
                self.log.error(&format!(
                    "the rate limits could not be read, the defaults apply: {error}"
                ));
                RateLimits::default()
            }
        }
    }
}

#[cfg(test)]
#[path = "rate_limit_source.tests.rs"]
mod tests;
