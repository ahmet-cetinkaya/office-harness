// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::{File, OpenOptions, TryLockError};

use private_fs::open_private_file;

use super::StartupError;
use crate::config::Config;

/// Holds the OS file lock of `server.lock` for as long as it lives. The OS
/// releases the lock when the process ends, however it ends.
#[derive(Debug)]
pub struct InstanceLock {
    _file: File,
}

/// Takes the single-instance lock of the data folder.
///
/// # Errors
/// `AlreadyRunning` when another server holds the lock, `DataFolder` when the
/// lock file cannot be opened.
pub fn acquire_single_instance(config: &Config) -> Result<InstanceLock, StartupError> {
    let file = open_private_file(
        &config.lock_path(),
        OpenOptions::new().read(true).write(true),
    )?;
    match file.try_lock() {
        Ok(()) => Ok(InstanceLock { _file: file }),
        Err(TryLockError::WouldBlock) => {
            Err(StartupError::AlreadyRunning(config.data_folder.clone()))
        }
        Err(TryLockError::Error(error)) => Err(StartupError::DataFolder(error)),
    }
}
