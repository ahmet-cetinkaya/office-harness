// SPDX-License-Identifier: AGPL-3.0-or-later

//! Files and folders that only the owner can read: mode 0600/0700 on Unix and
//! a protected access list on Windows.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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

/// Replaces the file with `contents` in one step: the bytes go to an
/// owner-only sibling file that then takes the place of `path`, so a reader
/// sees the old file or the new one, never a partial one.
///
/// # Errors
/// Returns the I/O error of the write, of the rename, or of the folder sync.
pub fn replace_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let sibling = sibling_path(path);
    let mut file = open_private_file(&sibling, OpenOptions::new().write(true).truncate(true))?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&sibling, path)?;
    sync_parent_folder(path)
}

/// Writes the folder entry of the rename to disk, so that after a crash the
/// new file cannot roll back to the old one.
#[cfg(unix)]
fn sync_parent_folder(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent)?.sync_all()
}

/// Windows has no folder sync; `MoveFileExW` with write-through is not used
/// because `fs::rename` does not offer it.
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn sync_parent_folder(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn sibling_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".new");
    PathBuf::from(name)
}

#[cfg(test)]
#[path = "lib.tests.rs"]
mod tests;
