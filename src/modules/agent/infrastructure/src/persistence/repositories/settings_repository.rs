// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::errors::AgentError;
use agent_domain::value_objects::{SettingKey, Settings};
use rusqlite::params;

use crate::persistence::{PersistenceError, SharedConnection};

pub struct SettingsRepository {
    connection: SharedConnection,
}

impl SettingsRepository {
    #[must_use]
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// The stored settings over the defaults, so a missing row means its default.
    ///
    /// # Errors
    /// A storage failure, or a stored row with an unknown key or a bad value.
    pub fn load(&self) -> Result<Settings, PersistenceError> {
        self.connection.with(|connection| {
            let mut statement = connection.prepare("SELECT key, value FROM agent_settings")?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let mut settings = Settings::default();
            for (name, value) in rows {
                let key = SettingKey::parse(&name)?;
                match settings.set_from_stored_text(key, &value) {
                    // A focus shortcut stored before a later change reserved
                    // its keys (0007 took Ctrl+Shift+H for the office hold)
                    // falls back to the default instead of stopping the
                    // server from starting. The jump key is the exception
                    // and is kept (0008 D6).
                    Err(AgentError::ReservedShortcut { .. })
                        if key == SettingKey::FocusShortcut => {}
                    stored => stored?,
                }
            }
            Ok(settings)
        })
    }

    /// Stores all settings in one transaction.
    ///
    /// # Errors
    /// A storage failure.
    pub fn save(&self, settings: &Settings) -> Result<(), PersistenceError> {
        self.connection.with(|connection| {
            let transaction = connection.transaction()?;
            for key in SettingKey::ALL {
                transaction.execute(
                    "INSERT INTO agent_settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                    params![key.as_str(), settings.value_of(key)],
                )?;
            }
            Ok(transaction.commit()?)
        })
    }
}
