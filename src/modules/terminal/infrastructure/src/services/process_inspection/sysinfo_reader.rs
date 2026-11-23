// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reads one process through `sysinfo`: the macOS reader. The start time is in
//! whole seconds, which is stable between reads. A Linux test builds this too,
//! to keep the command line form the same as the `/proc` reader gives.

use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind};
use terminal_application::common::interfaces::ReadError;
use terminal_application::common::models::ProcessIdentity;

pub fn read_identity(pid: u32) -> Result<ProcessIdentity, ReadError> {
    let mut system = System::new().map_err(|_| ReadError::NotReadable)?;
    let target = Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[target]),
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    let process = system.process(target).ok_or(ReadError::NotRunning)?;
    if process.status() == ProcessStatus::Zombie {
        return Err(ReadError::NotRunning);
    }
    let command_line = process
        .cmd()
        .iter()
        .map(|argument| argument.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    if command_line.is_empty() {
        return Err(ReadError::NotReadable);
    }
    let boot = super::machine_run().ok_or(ReadError::NotReadable)?;
    Ok(ProcessIdentity::new(
        pid,
        process.start_time(),
        &command_line,
        boot,
    ))
}
