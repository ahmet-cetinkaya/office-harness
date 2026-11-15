// SPDX-License-Identifier: AGPL-3.0-or-later

// Hides the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("office-harness failed to start: {error}");
        std::process::exit(1);
    }
}
