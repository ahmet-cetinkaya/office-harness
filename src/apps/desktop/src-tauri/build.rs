// SPDX-License-Identifier: AGPL-3.0-or-later

use tauri_build::{AppManifest, Attributes};

// The commands the webview may call. Each gets an `allow-<name>` permission
// that `capabilities/default.json` grants.
const WEBVIEW_COMMANDS: &[&str] = &[
    "watch_connection",
    "watch_events",
    "watch_terminal_frames",
    "send_request",
    "retry_connection",
    "start_server",
    "register_login_start",
    "login_start_status",
    "set_window_title",
    "export_templates_file",
    "import_templates_file",
];

fn main() {
    let manifest = AppManifest::new().commands(WEBVIEW_COMMANDS);
    if let Err(error) = tauri_build::try_build(Attributes::new().app_manifest(manifest)) {
        panic!("tauri build failed: {error:#}");
    }
}
