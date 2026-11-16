// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;
use wire::PROTOCOL_VERSION;

/// The part of `server.json` that the app uses.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ServerFile {
    pub port: u16,
    pub protocol: u16,
    pub version: String,
}

/// The server and the app cannot work together as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incompatibility {
    pub server_version: String,
    pub app_version: String,
    /// A different version can be asked to restart; a different protocol
    /// cannot even be spoken to.
    pub can_restart: bool,
}

impl ServerFile {
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn incompatibility(&self, app_version: &str) -> Option<Incompatibility> {
        let protocol_matches = self.protocol == PROTOCOL_VERSION;
        if protocol_matches && self.version == app_version {
            return None;
        }
        Some(Incompatibility {
            server_version: self.version.clone(),
            app_version: app_version.to_owned(),
            can_restart: protocol_matches,
        })
    }
}

#[cfg(test)]
#[path = "server_file.tests.rs"]
mod tests;
