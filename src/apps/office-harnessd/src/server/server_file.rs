// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io;
use std::path::Path;

use private_fs::replace_private_file;
use serde_json::json;
use wire::PROTOCOL_VERSION;

/// Writes `server.json` owner-only, in one step, so a client never reads half
/// of it. `start_nonce` tells a client whether the file belongs to this start
/// of the server or to an older one.
///
/// # Errors
/// The I/O error of the write.
pub fn write_server_file(path: &Path, port: u16, start_nonce: &str) -> io::Result<()> {
    let contents = json!({
        "port": port,
        "pid": std::process::id(),
        "protocol": PROTOCOL_VERSION,
        "version": env!("CARGO_PKG_VERSION"),
        "start_nonce": start_nonce,
    });
    replace_private_file(path, contents.to_string().as_bytes())
}

#[cfg(test)]
#[path = "server_file.tests.rs"]
mod tests;
