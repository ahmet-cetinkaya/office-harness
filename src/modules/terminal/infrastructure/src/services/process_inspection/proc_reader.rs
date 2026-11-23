// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reads one Linux process from `/proc`. `sysinfo` keeps the start time in
//! whole seconds and hides the raw clock ticks, so the exact start time and
//! the command line come from here (plan D2).

use std::fs;
use std::io::ErrorKind;

use terminal_application::common::interfaces::ReadError;
use terminal_application::common::models::ProcessIdentity;

const BOOT_ID_PATH: &str = "/proc/sys/kernel/random/boot_id";

/// Field 22 of `/proc/<pid>/stat` is the start time; the fields after the
/// command name start with field 3, the state.
const START_TIME_FIELD: usize = 22;
const FIRST_FIELD_AFTER_COMMAND: usize = 3;
const ZOMBIE_STATE: &str = "Z";
const DEAD_STATE: &str = "X";

/// The values of `/proc/<pid>/stat` that the identity needs.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct StatFields {
    pub(super) state: String,
    pub(super) started_at: u64,
}

pub fn boot_id() -> Option<String> {
    let text = fs::read_to_string(BOOT_ID_PATH).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

pub fn read_identity(pid: u32) -> Result<ProcessIdentity, ReadError> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|error| not_running_if_gone(&error))?;
    let fields = parse_stat(&stat).ok_or(ReadError::NotReadable)?;
    if fields.state == ZOMBIE_STATE || fields.state == DEAD_STATE {
        return Err(ReadError::NotRunning);
    }
    let command_line = read_command_line(pid)?;
    let boot = boot_id().ok_or(ReadError::NotReadable)?;
    Ok(ProcessIdentity::new(
        pid,
        fields.started_at,
        &command_line,
        boot,
    ))
}

/// The arguments joined with a single space, the form that `sysinfo` gives on
/// macOS too, so the stored and the live line have one form. An empty line
/// is not readable.
fn read_command_line(pid: u32) -> Result<String, ReadError> {
    let bytes =
        fs::read(format!("/proc/{pid}/cmdline")).map_err(|error| not_running_if_gone(&error))?;
    let line = String::from_utf8_lossy(&bytes)
        .trim_end_matches('\0')
        .replace('\0', " ");
    if line.is_empty() {
        return Err(ReadError::NotReadable);
    }
    Ok(line)
}

fn not_running_if_gone(error: &std::io::Error) -> ReadError {
    if error.kind() == ErrorKind::NotFound {
        ReadError::NotRunning
    } else {
        ReadError::NotReadable
    }
}

/// The command name sits in parentheses and may hold spaces and parentheses
/// itself, so the fields start after the last closing parenthesis.
pub(super) fn parse_stat(stat: &str) -> Option<StatFields> {
    let after_command = &stat[stat.rfind(')')? + 1..];
    let mut fields = after_command.split_whitespace();
    let state = fields.next()?.to_owned();
    let started_at = fields
        .nth(START_TIME_FIELD - FIRST_FIELD_AFTER_COMMAND - 1)?
        .parse()
        .ok()?;
    Some(StatFields { state, started_at })
}

#[cfg(test)]
#[path = "proc_reader.tests.rs"]
mod tests;
