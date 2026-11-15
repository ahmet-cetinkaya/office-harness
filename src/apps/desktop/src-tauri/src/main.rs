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

fn main() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let connection = Connection::new(DataFolder::locate()?);
            app.manage(connection.clone());
            tauri::async_runtime::spawn(connection.run());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::watch_connection,
            commands::watch_events,
            commands::watch_terminal_frames,
            commands::send_request,
            commands::retry_connection,
            commands::start_server,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("office-harness failed to start: {error}");
        std::process::exit(1);
    }
}
