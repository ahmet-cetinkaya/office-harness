// SPDX-License-Identifier: AGPL-3.0-or-later

//! The tasks that push the connection state and the server's events to the
//! webview's channels. Each ends when the webview stops listening.

use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::sync::{broadcast, watch};

use crate::connection::ConnectionState;

/// Sends the current state, then each change.
pub fn forward_state(
    mut states: watch::Receiver<ConnectionState>,
    channel: Channel<ConnectionState>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let current = states.borrow_and_update().clone();
            if channel.send(current).is_err() || states.changed().await.is_err() {
                return;
            }
        }
    });
}

/// A webview that falls behind loses the oldest events; it reloads its data
/// whenever the state becomes connected again.
pub fn forward_events(mut events: broadcast::Receiver<Value>, channel: Channel<Value>) {
    tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => {
                    if channel.send(event).is_err() {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });
}

/// Sends each binary terminal frame to the webview as raw bytes. A webview
/// that falls behind loses the oldest frames and sees a gap; the server's own
/// slow-client rule disconnects it first in practice.
pub fn forward_frames(
    mut frames: broadcast::Receiver<Vec<u8>>,
    channel: Channel<InvokeResponseBody>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            match frames.recv().await {
                Ok(frame) => {
                    if channel.send(InvokeResponseBody::Raw(frame)).is_err() {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });
}
