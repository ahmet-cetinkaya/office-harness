// SPDX-License-Identifier: AGPL-3.0-or-later

//! The commands the webview may call. The list is also in `build.rs` and in
//! `capabilities/default.json`; a command that is in none of them cannot be
//! called.

// Tauri hands each command its managed state by value.
#![allow(clippy::needless_pass_by_value)]

use std::time::Duration;

use serde_json::{Map, Value};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{State, WebviewWindow};

use crate::api_error::{
    ApiError, LOGIN_START_FAILED, START_FAILED, START_TIMEOUT, WINDOW_TITLE_FAILED,
};
use crate::channels;
use crate::connection::{Connection, ConnectionState};
use crate::server_process::{self, LoginStartError};

/// How long "start server" waits for the new server to answer.
const START_WAIT: Duration = Duration::from_secs(15);

/// The window title without a count; it is the one in `tauri.conf.json`.
const APP_TITLE: &str = "office-harness";

/// The title that shows how many agents need the user (0008 D7).
fn window_title(count: u32) -> String {
    if count == 0 {
        APP_TITLE.to_owned()
    } else {
        format!("{APP_TITLE} ({count})")
    }
}

#[tauri::command]
pub fn watch_connection(connection: State<'_, Connection>, on_state: Channel<ConnectionState>) {
    channels::forward_state(connection.subscribe_state(), on_state);
}

#[tauri::command]
pub fn watch_events(connection: State<'_, Connection>, on_event: Channel<Value>) {
    channels::forward_events(connection.subscribe_events(), on_event);
}

#[tauri::command]
pub fn watch_terminal_frames(
    connection: State<'_, Connection>,
    on_frame: Channel<InvokeResponseBody>,
) {
    channels::forward_frames(connection.subscribe_frames(), on_frame);
}

#[tauri::command]
pub async fn send_request(
    connection: State<'_, Connection>,
    kind: String,
    fields: Option<Map<String, Value>>,
) -> Result<Value, ApiError> {
    connection.request(kind, fields.unwrap_or_default()).await
}

#[tauri::command]
pub fn retry_connection(connection: State<'_, Connection>) {
    connection.retry_now();
}

#[tauri::command]
pub async fn start_server(connection: State<'_, Connection>) -> Result<(), ApiError> {
    server_process::start_detached()
        .map_err(|error| ApiError::new(START_FAILED, error.to_string()))?;
    connection.retry_now();
    if connection.wait_until_settled(START_WAIT).await {
        return Ok(());
    }
    Err(ApiError::new(
        START_TIMEOUT,
        "The server started but did not answer in time. Its log file in the office data folder says why.",
    ))
}

/// The sidecar runs for a moment and may touch the OS, so it must not hold
/// up the command threads of the UI.
async fn run_login_start<Answer: Send + 'static>(
    work: fn() -> Result<Answer, LoginStartError>,
) -> Result<Answer, ApiError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| ApiError::new(LOGIN_START_FAILED, error.to_string()))?
        .map_err(|error| ApiError::new(LOGIN_START_FAILED, error.to_string()))
}

#[tauri::command]
pub async fn register_login_start() -> Result<(), ApiError> {
    run_login_start(server_process::register_login_start).await
}

#[tauri::command]
pub async fn login_start_status() -> Result<bool, ApiError> {
    run_login_start(server_process::is_login_start_registered).await
}

/// Takes only a number, so the webview cannot set any other title text, and
/// needs no broad window permission (0008 D7).
#[tauri::command]
pub fn set_window_title(window: WebviewWindow, count: u32) -> Result<(), ApiError> {
    window
        .set_title(&window_title(count))
        .map_err(|error| ApiError::new(WINDOW_TITLE_FAILED, error.to_string()))
}

#[cfg(test)]
#[path = "commands.tests.rs"]
mod tests;
