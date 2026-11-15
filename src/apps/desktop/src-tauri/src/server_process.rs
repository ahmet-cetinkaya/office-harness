// SPDX-License-Identifier: AGPL-3.0-or-later

//! Starts the office server so that it outlives the app (FR-001, FR-026).
//! Starting through the login item and running `register` arrive in T11.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use thiserror::Error;

const SIDECAR_NAME: &str = "office-harnessd";

#[derive(Debug, Error)]
pub enum StartServerError {
    #[error("the app cannot find its own folder: {0}")]
    NoAppFolder(io::Error),
    #[error("the server program could not be started: {0}")]
    Spawn(io::Error),
}

/// Starts the sidecar that sits next to the app, as a process of its own.
pub fn start_detached() -> Result<(), StartServerError> {
    // The app may be started through a symlink; the sidecar sits next to the real file.
    let app = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(StartServerError::NoAppFolder)?;
    let mut command = Command::new(sidecar_path(&app));
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    let mut child = command.spawn().map_err(StartServerError::Spawn)?;
    // The server keeps running after the app. This only collects the exit
    // status of a server that stops while the app is still open.
    thread::spawn(move || child.wait());
    Ok(())
}

fn sidecar_path(app: &Path) -> PathBuf {
    let file_name = format!("{SIDECAR_NAME}{}", std::env::consts::EXE_SUFFIX);
    app.with_file_name(file_name)
}

/// A new process group, so a signal to the app's group (a terminal's Ctrl+C)
/// does not reach the server.
#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(test)]
#[path = "server_process.tests.rs"]
mod tests;
