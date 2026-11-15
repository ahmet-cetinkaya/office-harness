// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::Notify;

/// The request type that asks the server to stop so that a new one can take
/// its place (plan D13). It carries no body.
pub const RESTART_REQUEST: &str = "server.restart";

/// Carries a `server.restart` request from a connection to the code that
/// runs the server. A request that arrives before anyone waits is kept.
#[derive(Clone, Default)]
pub struct RestartSignal(Arc<Notify>);

impl RestartSignal {
    pub fn request(&self) {
        self.0.notify_one();
    }

    /// Completes when a restart was requested.
    pub async fn requested(&self) {
        self.0.notified().await;
    }
}
