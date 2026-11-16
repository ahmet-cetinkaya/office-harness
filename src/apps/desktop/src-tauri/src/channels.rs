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

/// Not a server event: the webview missed events and must reload. The
/// `client.` prefix cannot clash with a contract event name.
const RESYNC_EVENT_TYPE: &str = "client.resync";

/// The server's dropped-output frame kind (client protocol, terminal frames).
const FRAME_KIND_DROPPED: u8 = 0x03;
const AGENT_ID_BYTES: usize = 16;

/// A dropped-output frame for the nil agent id, which the webview delivers to
/// every open terminal because the lost frames name no agent.
fn lagged_frame() -> Vec<u8> {
    let mut frame = vec![0; 1 + AGENT_ID_BYTES];
    frame[0] = FRAME_KIND_DROPPED;
    frame
}

fn resync_event() -> Value {
    serde_json::json!({ "type": RESYNC_EVENT_TYPE })
}

/// A webview that falls behind loses the oldest events; it gets a resync event
/// and reloads its data, and it also reloads whenever the state becomes
/// connected again.
pub fn forward_events(mut events: broadcast::Receiver<Value>, channel: Channel<Value>) {
    tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => {
                    if channel.send(event).is_err() {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    if channel.send(resync_event()).is_err() {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });
}

/// Sends each binary terminal frame to the webview as raw bytes. A webview
/// that falls behind loses the oldest frames and gets a dropped-output frame,
/// so its open terminals show the dropped-output notice.
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
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let notice = InvokeResponseBody::Raw(lagged_frame());
                    if channel.send(notice).is_err() {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });
}

#[cfg(test)]
#[path = "channels.tests.rs"]
mod tests;
