// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::FailureReporter;
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
        if error.is_storage_failure() {
            self.report_disk_failure(&error.to_string());
        }
    }

    /// Raises an alert for a failure that a module already classified as the
    /// disk's: logs it and tells the clients.
    pub fn report_disk_failure(&self, message: &str) {
        let message = message.to_owned();
        self.log.error(&format!("{STORAGE_ERROR_EVENT}: {message}"));
        // No subscriber is a normal state: the alert is still in the log.
        let _ = self.sender.send(StorageAlert { message });
    }
}

impl FailureReporter for StorageAlerts {
    fn report(&self, error: &PortError) {
        match error {
            PortError::Storage {
                message,
                is_disk_failure: true,
            } => self.report_disk_failure(message),
            other => self
                .log
                .error(&format!("a background write failed: {other}")),
        }
    }

    fn note(&self, message: &str) {
        self.log.error(message);
    }
}
