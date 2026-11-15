// SPDX-License-Identifier: AGPL-3.0-or-later

use tauri_build::{AppManifest, Attributes};

// The commands the webview may call. Each gets an `allow-<name>` permission
// that `capabilities/default.json` grants.
const WEBVIEW_COMMANDS: &[&str] = &[
    "watch_connection",
    "watch_events",
    "send_request",
    "retry_connection",
    "start_server",
];

fn main() {
    let manifest = AppManifest::new().commands(WEBVIEW_COMMANDS);
    if let Err(error) = tauri_build::try_build(Attributes::new().app_manifest(manifest)) {
        panic!("tauri build failed: {error:#}");
    }
}
