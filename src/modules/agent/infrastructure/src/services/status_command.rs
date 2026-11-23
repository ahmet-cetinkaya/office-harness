// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runs a CLI's sign-in status command (0019 D6): no shell, no terminal, a
//! null input, exactly the environment given, a deadline that ends the
//! whole process tree, and at most the output cap kept in memory. Nothing
//! is written to a file or a log.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use agent_application::common::models::StatusRunResult;
use agent_application::common::sign_in_check::STATUS_OUTPUT_CAP_BYTES;

use crate::services::bounded_child::{
    bounded_reader, end_tree, exited_output, own_process_group, wait_until,
};
use crate::services::login_environment::Environment;

/// Runs `program` with `arguments` in `folder`. The answer holds what the
/// command printed on both streams, standard output first, cut to the cap.
#[must_use]
pub fn run_status_command(
    program: &Path,
    arguments: &[String],
    folder: &Path,
    environment: &Environment,
    timeout: Duration,
) -> StatusRunResult {
    let deadline = Instant::now() + timeout;
    let mut command = Command::new(program);
    command
        .args(arguments)
        .current_dir(folder)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    own_process_group(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return StatusRunResult::CouldNotRun(error.to_string()),
    };
    let stdout = child
        .stdout
        .take()
        .map(|pipe| bounded_reader(pipe, STATUS_OUTPUT_CAP_BYTES));
    let stderr = child
        .stderr
        .take()
        .map(|pipe| bounded_reader(pipe, STATUS_OUTPUT_CAP_BYTES));
    if !wait_until(&mut child, deadline) {
        end_tree(&mut child);
        return StatusRunResult::TimedOut;
    }
    let (stdout, stderr) = exited_output(&mut child, stdout.as_ref(), stderr.as_ref());
    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => return StatusRunResult::CouldNotRun(error.to_string()),
    };
    let mut output = stdout.map_or_else(Vec::new, |(bytes, _)| bytes);
    // Each stream's last line stays a line of its own.
    if !output.is_empty() && !output.ends_with(b"\n") {
        output.push(b'\n');
    }
    output.extend_from_slice(&stderr);
    output.truncate(STATUS_OUTPUT_CAP_BYTES);
    StatusRunResult::Ended {
        exit_code: status.code(),
        output: String::from_utf8_lossy(&output).into_owned(),
    }
}

#[cfg(test)]
#[path = "status_command.tests.rs"]
mod tests;
