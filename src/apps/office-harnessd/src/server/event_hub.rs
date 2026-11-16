// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use tokio::sync::broadcast;
use wire::Event;

use crate::bootstrap::{STORAGE_ERROR_EVENT, ServerLog, StorageAlert};

const EVENT_BACKLOG: usize = 256;

/// Fans server events out to every authenticated connection. A connection
/// that falls behind by more than the backlog is closed with 1011, so that
/// its client reconnects and reloads the current state.
#[derive(Clone)]
pub struct EventHub {
    sender: broadcast::Sender<Arc<str>>,
}

impl EventHub {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BACKLOG);
        Self { sender }
    }

    pub fn publish(&self, event: &Event) {
        // No subscriber is a normal state: nobody is connected.
        let _ = self.sender.send(Arc::from(event.encode()));
    }

    /// Receives each event from now on, as its JSON text frame.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<str>> {
        self.sender.subscribe()
    }

    /// Publishes each storage alert as a `storage.error` event until the
    /// alert source is dropped.
    pub fn forward_storage_alerts(
        &self,
        mut alerts: broadcast::Receiver<StorageAlert>,
        log: ServerLog,
    ) {
        let hub = self.clone();
        tokio::spawn(async move {
            loop {
                match alerts.recv().await {
                    Ok(alert) => hub.publish(&storage_error_event(alert)),
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        log.error(&format!("{missed} storage alerts were not sent to clients"));
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
}

impl Default for EventHub {
    fn default() -> Self {
        Self::new()
    }
}

fn storage_error_event(alert: StorageAlert) -> Event {
    let mut fields = serde_json::Map::new();
    fields.insert("message".to_owned(), alert.message.into());
    Event::new(STORAGE_ERROR_EVENT, fields)
}
