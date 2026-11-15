// SPDX-License-Identifier: AGPL-3.0-or-later

//! Files and folders that only the owner can read: mode 0600/0700 on Unix and
//! a protected access list on Windows.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

#[cfg(any(windows, test))]
mod windows_access_list;

#[cfg(unix)]
mod unix_mode;
#[cfg(unix)]
use unix_mode as platform;
#[cfg(windows)]
use windows_access_list as platform;

/// Creates the folder and its missing parents, and makes the folder itself
/// owner-only even when it already existed with wider access.
///
/// # Errors
/// Returns the I/O error of the folder creation or of the access change.
pub fn create_private_dir_all(folder: &Path) -> io::Result<()> {
    platform::create_dir_all_private(folder)?;
    platform::restrict_folder(folder)
}

/// Opens the file with the given options, creating it when missing, and makes
/// it owner-only even when it already existed with wider access.
///
/// # Errors
/// Returns the I/O error of the open or of the access change.
pub fn open_private_file(path: &Path, options: &mut OpenOptions) -> io::Result<File> {
    options.create(true);
    platform::apply_creation_mode(options);
    let file = options.open(path)?;
    platform::restrict_file(path)?;
    Ok(file)
}

#[cfg(test)]
#[path = "lib.tests.rs"]
mod tests;
