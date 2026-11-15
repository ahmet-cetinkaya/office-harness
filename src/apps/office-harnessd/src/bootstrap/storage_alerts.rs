// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_infrastructure::persistence::PersistenceError;
use tokio::sync::broadcast;

use super::ServerLog;

/// The protocol event type that tells clients the disk failed (contract).
pub const STORAGE_ERROR_EVENT: &str = "storage.error";

const ALERT_BACKLOG: usize = 16;

/// The body of a `storage.error` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageAlert {
    pub message: String,
}

/// Turns failed storage writes into `storage.error` events and log lines. The
/// failing request gets its error back, and the server keeps running, so the
/// agents keep running.
#[derive(Clone)]
pub struct StorageAlerts {
    sender: broadcast::Sender<StorageAlert>,
    log: ServerLog,
}

impl StorageAlerts {
    #[must_use]
    pub fn new(log: ServerLog) -> Self {
        let (sender, _) = broadcast::channel(ALERT_BACKLOG);
        Self { sender, log }
    }

    /// Receives each alert from now on. The event fan-out subscribes here.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<StorageAlert> {
        self.sender.subscribe()
    }

    /// Passes the result through, and raises an alert when it failed because of
    /// the disk.
    ///
    /// # Errors
    /// The same error that `result` held.
    pub fn guard<T>(&self, result: Result<T, PersistenceError>) -> Result<T, PersistenceError> {
        if let Err(error) = &result {
            self.report(error);
        }
        result
    }

    fn report(&self, error: &PersistenceError) {
        if !error.is_storage_failure() {
            return;
        }
        let message = error.to_string();
        self.log.error(&format!("{STORAGE_ERROR_EVENT}: {message}"));
        // No subscriber is a normal state: the alert is still in the log.
        let _ = self.sender.send(StorageAlert { message });
    }
}
