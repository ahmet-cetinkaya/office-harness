// SPDX-License-Identifier: AGPL-3.0-or-later

//! Starts the office server so that it outlives the app (FR-001, FR-026), and
//! asks the sidecar to register itself as a login item (FR-002, plan D13).

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use thiserror::Error;
use wire::LOGIN_START_NOT_REGISTERED_EXIT_CODE;

const SIDECAR_NAME: &str = "office-harnessd";
const REGISTER_ARGUMENT: &str = "register";
/// Registers only on a first start or a new version, and never after the
/// user's `unregister` (plan D13).
const AT_APP_START_ARGUMENT: &str = "--at-app-start";
const STATUS_ARGUMENT: &str = "status";

#[derive(Debug, Error)]
pub enum StartServerError {
    #[error("the app cannot find its own folder: {0}")]
    NoAppFolder(io::Error),
    #[error("the server program could not be started: {0}")]
    Spawn(io::Error),
}

#[derive(Debug, Error)]
pub enum LoginStartError {
    #[error("the app cannot find its own folder: {0}")]
    NoAppFolder(io::Error),
    #[error("the server program could not be run: {0}")]
    Run(io::Error),
    #[error("the server program could not change the login item: {0}")]
    Failed(String),
}

/// The sidecar sits next to the app. The app may be started through a
/// symlink, so the path comes from the real file.
fn locate_sidecar() -> io::Result<PathBuf> {
    let app = std::env::current_exe().and_then(std::fs::canonicalize)?;
    Ok(sidecar_path(&app))
}

/// Starts the sidecar that sits next to the app, as a process of its own.
pub fn start_detached() -> Result<(), StartServerError> {
    let mut command = Command::new(locate_sidecar().map_err(StartServerError::NoAppFolder)?);
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

/// Asks the sidecar to copy itself to a stable place and start that copy at
/// login. It is safe to repeat, and a new app version makes a new copy.
pub fn register_login_start() -> Result<(), LoginStartError> {
    let answer = run_sidecar(&[REGISTER_ARGUMENT])?;
    answer_to_success(&answer)
}

/// As `register_login_start`, but only on the first start of the app or of a
/// new version, and not after `office-harnessd unregister` (plan D13).
pub fn refresh_login_start() -> Result<(), LoginStartError> {
    let answer = run_sidecar(&[REGISTER_ARGUMENT, AT_APP_START_ARGUMENT])?;
    answer_to_success(&answer)
}

/// Whether a login item for the server exists.
pub fn is_login_start_registered() -> Result<bool, LoginStartError> {
    let answer = run_sidecar(&[STATUS_ARGUMENT])?;
    answer_to_registered(&answer)
}

/// What a finished sidecar left behind.
struct SidecarAnswer {
    exit_code: Option<i32>,
    stderr: String,
}

fn run_sidecar(arguments: &[&str]) -> Result<SidecarAnswer, LoginStartError> {
    let program = locate_sidecar().map_err(LoginStartError::NoAppFolder)?;
    let output = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(LoginStartError::Run)?;
    Ok(SidecarAnswer {
        exit_code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

fn answer_to_success(answer: &SidecarAnswer) -> Result<(), LoginStartError> {
    if answer.exit_code == Some(0) {
        return Ok(());
    }
    Err(failure(answer))
}

fn answer_to_registered(answer: &SidecarAnswer) -> Result<bool, LoginStartError> {
    match answer.exit_code {
        Some(0) => Ok(true),
        Some(code) if code == i32::from(LOGIN_START_NOT_REGISTERED_EXIT_CODE) => Ok(false),
        _ => Err(failure(answer)),
    }
}

fn failure(answer: &SidecarAnswer) -> LoginStartError {
    let reason = if answer.stderr.is_empty() {
        "it gave no reason"
    } else {
        &answer.stderr
    };
    LoginStartError::Failed(reason.to_owned())
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
