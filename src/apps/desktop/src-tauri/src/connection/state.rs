// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Serialize;
use ts_rs::TS;

use super::server_file::Incompatibility;

/// What the webview shows while it has no usable connection, and the one state
/// in which it may show the office.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/core/api/generated/")]
pub enum ConnectionState {
    Loading,
    /// The server has no free handshake slot; the app tries again by itself.
    Busy,
    NotReachable,
    NotAuthorized,
    VersionMismatch {
        server_version: String,
        app_version: String,
        /// False when the protocol differs: the app refuses to talk to that
        /// server at all, so it cannot ask it to restart.
        can_restart: bool,
    },
    Connected,
}

impl ConnectionState {
    /// The states in which waiting for the server is over.
    pub fn is_settled(&self) -> bool {
        !matches!(self, Self::Loading | Self::Busy | Self::NotReachable)
    }
}

impl From<Incompatibility> for ConnectionState {
    fn from(incompatibility: Incompatibility) -> Self {
        Self::VersionMismatch {
            server_version: incompatibility.server_version,
            app_version: incompatibility.app_version,
            can_restart: incompatibility.can_restart,
        }
    }
}
