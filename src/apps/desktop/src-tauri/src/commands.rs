// SPDX-License-Identifier: AGPL-3.0-or-later

//! The commands the webview may call. The list is also in `build.rs` and in
//! `capabilities/default.json`; a command that is in none of them cannot be
//! called.

// Tauri hands each command its managed state by value.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use serde_json::{Map, Value};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::api_error::{
    ApiError, LOGIN_START_FAILED, START_FAILED, START_TIMEOUT, TEMPLATE_FILE_FAILED,
    WINDOW_TITLE_FAILED,
};
use crate::channels;
use crate::connection::{Connection, ConnectionState};
use crate::server_process::{self, LoginStartError};
use crate::template_files;

/// How long "start server" waits for the new server to answer.
const START_WAIT: Duration = Duration::from_secs(15);

/// The window title without a count; it is the one in `tauri.conf.json`.
const APP_TITLE: &str = "office-harness";

/// The name that the save dialog offers for the export file.
const EXPORT_FILE_NAME: &str = "office-harness-templates.json";

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

/// What an export did. A closed dialog is not a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportOutcome {
    Saved,
    Cancelled,
}

fn template_file_failure(error: &template_files::TemplateFileError) -> ApiError {
    ApiError::new(TEMPLATE_FILE_FAILED, error.to_string())
}

/// `None` when the user closes the dialog. A path the dialog cannot name as a
/// local path is a failure.
fn picked_path(picked: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<PathBuf>, ApiError> {
    picked
        .map(|file| {
            file.into_path()
                .map_err(|error| ApiError::new(TEMPLATE_FILE_FAILED, error.to_string()))
        })
        .transpose()
}

/// Takes no path: it writes only to the path the user picks in its own save
/// dialog, so the webview cannot write any other file (0009 D6).
#[tauri::command]
pub async fn export_templates_file(
    app: AppHandle,
    contents: String,
) -> Result<ExportOutcome, ApiError> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app
            .dialog()
            .file()
            .add_filter("Template file", &["json"])
            .set_file_name(EXPORT_FILE_NAME)
            .blocking_save_file();
        let Some(path) = picked_path(picked)? else {
            return Ok(ExportOutcome::Cancelled);
        };
        template_files::write_export_file(&path, &contents)
            .map_err(|error| template_file_failure(&error))?;
        Ok(ExportOutcome::Saved)
    })
    .await
    .map_err(|error| ApiError::new(TEMPLATE_FILE_FAILED, error.to_string()))?
}

/// Takes no path: it reads only the path the user picks in its own open
/// dialog. The answer is the text, or `None` when the user closes the dialog.
#[tauri::command]
pub async fn import_templates_file(app: AppHandle) -> Result<Option<String>, ApiError> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app
            .dialog()
            .file()
            .add_filter("Template file", &["json"])
            .blocking_pick_file();
        picked_path(picked)?
            .map(|path| {
                template_files::read_import_file(&path)
                    .map_err(|error| template_file_failure(&error))
            })
            .transpose()
    })
    .await
    .map_err(|error| ApiError::new(TEMPLATE_FILE_FAILED, error.to_string()))?
}

#[cfg(test)]
#[path = "commands.tests.rs"]
mod tests;
