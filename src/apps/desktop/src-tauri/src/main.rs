// SPDX-License-Identifier: AGPL-3.0-or-later

// Hides the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api_error;
mod channels;
mod commands;
mod connection;
mod server_process;

use connection::{Connection, DataFolder};
use tauri::Manager;

/// A failure here must not stop the app; the access panel shows the state
/// and offers the action again.
fn refresh_login_start_quietly() {
    if let Err(error) = server_process::refresh_login_start() {
        eprintln!("office-harness could not register the server for login: {error}");
    }
}

fn main() {
    let result = tauri::Builder::default()
        // OS notifications for the agent events (0003, D7).
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let connection = Connection::new(DataFolder::locate()?);
            app.manage(connection.clone());
            tauri::async_runtime::spawn(connection.run());
            // Registers on the first start and after an upgrade, unless the
            // user turned login start off (plan D13); the UI does not wait.
            tauri::async_runtime::spawn_blocking(refresh_login_start_quietly);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::watch_connection,
            commands::watch_events,
            commands::watch_terminal_frames,
            commands::send_request,
            commands::retry_connection,
            commands::start_server,
            commands::register_login_start,
            commands::login_start_status,
            commands::set_window_title,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("office-harness failed to start: {error}");
        std::process::exit(1);
    }
}
