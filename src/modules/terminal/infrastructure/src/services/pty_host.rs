// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::{Read, Write};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::models::SpawnTerminalRequest;
use terminal_domain::services::is_batch_script;
use terminal_domain::value_objects::TerminalSize;

use crate::services::npm_shim::resolve_npm_shim;

const BATCH_INTERPRETER: &str = "cmd.exe";
const SYSTEM_ROOT_VARIABLE: &str = "SystemRoot";
const SYSTEM_FOLDER: &str = "System32";
/// `/d` skips the interpreter's auto-run commands, `/c` runs the script and ends.
const BATCH_INTERPRETER_FLAGS: [&str; 2] = ["/d", "/c"];

/// The pieces of one child that runs in a PTY.
pub struct SpawnedPty {
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn Child + Send + Sync>,
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    /// True when `cmd.exe` runs the program, which asks a question when the
    /// interrupt key reaches a batch file.
    pub runs_through_batch_interpreter: bool,
}

/// Starts the command in a new PTY with exactly the environment of the
/// request. The caller resolves the program on the login `PATH` first.
///
/// # Errors
/// `SpawnFailed` with the reason when the PTY or the child cannot start.
pub fn spawn_in_pty(request: &SpawnTerminalRequest) -> Result<SpawnedPty, TerminalRuntimeError> {
    let pair = native_pty_system()
        .openpty(pty_size(request.size))
        .map_err(spawn_failed)?;
    let (program, arguments) = launch_command(request);
    let runs_through_batch_interpreter = is_batch_interpreter(&program);
    let mut builder = CommandBuilder::new(program);
    builder.args(arguments);
    builder.cwd(&request.working_folder);
    builder.env_clear();
    for (name, value) in &request.environment {
        builder.env(name, value);
    }
    let child = pair.slave.spawn_command(builder).map_err(spawn_failed)?;
    // The child holds the slave side now. Keeping ours open would stop the
    // reader from ever seeing the end of the output.
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().map_err(spawn_failed)?;
    let writer = pair.master.take_writer().map_err(spawn_failed)?;
    Ok(SpawnedPty {
        master: pair.master,
        child,
        reader,
        writer,
        runs_through_batch_interpreter,
    })
}

/// On Windows an npm shim starts its interpreter directly when it can, so no
/// `cmd.exe` stands between the stop key and the CLI. Everything else follows
/// `launch_line`.
fn launch_command(request: &SpawnTerminalRequest) -> (String, Vec<String>) {
    let is_batch_on_windows = cfg!(windows) && is_batch_script(&request.program);
    let direct = if is_batch_on_windows {
        resolve_npm_shim(&request.program, &request.arguments, &request.environment)
    } else {
        None
    };
    if let Some(launch) = direct {
        return launch;
    }
    launch_line(
        &request.program,
        &request.arguments,
        cfg!(windows).then(|| batch_interpreter(&request.environment)),
    )
}

/// The batch route starts `cmd.exe` by its path; a resolved npm shim and
/// every other program start as themselves.
fn is_batch_interpreter(program: &str) -> bool {
    program.to_ascii_lowercase().ends_with(BATCH_INTERPRETER)
}

#[must_use]
pub fn pty_size(size: TerminalSize) -> PtySize {
    PtySize {
        rows: size.rows().get(),
        cols: size.columns().get(),
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// A batch script on Windows runs through `cmd.exe`, because the OS cannot
/// start it directly. `batch_interpreter` is `Some` only on Windows. Every
/// other command runs as it is.
fn launch_line(
    program: &str,
    arguments: &[String],
    batch_interpreter: Option<String>,
) -> (String, Vec<String>) {
    let Some(interpreter) = batch_interpreter.filter(|_| is_batch_script(program)) else {
        return (program.to_owned(), arguments.to_vec());
    };
    let mut interpreter_arguments: Vec<String> = BATCH_INTERPRETER_FLAGS
        .iter()
        .map(|flag| (*flag).to_owned())
        .collect();
    interpreter_arguments.push(program.to_owned());
    interpreter_arguments.extend_from_slice(arguments);
    (interpreter, interpreter_arguments)
}

/// The interpreter by its full path in the system folder, so a `cmd.exe` in
/// the working folder or on a changed `PATH` is never the one that runs. The
/// system root comes from the child's environment, then from the server's.
fn batch_interpreter(environment: &[(String, String)]) -> String {
    let system_root = environment
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(SYSTEM_ROOT_VARIABLE))
        .map(|(_, value)| value.clone())
        .or_else(|| std::env::var(SYSTEM_ROOT_VARIABLE).ok());
    match system_root {
        Some(root) => format!("{root}\\{SYSTEM_FOLDER}\\{BATCH_INTERPRETER}"),
        None => BATCH_INTERPRETER.to_owned(),
    }
}

fn spawn_failed(error: impl std::fmt::Display) -> TerminalRuntimeError {
    TerminalRuntimeError::SpawnFailed(error.to_string())
}

#[cfg(test)]
#[path = "pty_host.tests.rs"]
mod tests;
