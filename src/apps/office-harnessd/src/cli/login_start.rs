// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{CliError, LoginItem};

/// Under the data folder; one folder for each version, so that a running
/// older server keeps its file while a newer copy is registered.
const COPIES_FOLDER: &str = "bin";
const PROGRAM_NAME: &str = "office-harnessd";
const PARTIAL_EXTENSION: &str = "partial";

/// Starts a stable copy of the server at login. The copy exists because the
/// running program may sit in a place that goes away: an AppImage mount, or an
/// app folder that the user moves (research R4).
pub struct LoginStart<'a> {
    data_folder: &'a Path,
    version: &'a str,
    login_item: &'a dyn LoginItem,
}

impl<'a> LoginStart<'a> {
    #[must_use]
    pub fn new(data_folder: &'a Path, version: &'a str, login_item: &'a dyn LoginItem) -> Self {
        Self {
            data_folder,
            version,
            login_item,
        }
    }

    /// Where the copy of this version lives.
    #[must_use]
    pub fn installed_program(&self) -> PathBuf {
        self.copies_folder()
            .join(self.version)
            .join(format!("{PROGRAM_NAME}{}", std::env::consts::EXE_SUFFIX))
    }

    /// Copies `running_program` to the versioned location unless a copy with
    /// the same bytes is there, then points the login item at the copy. Runs
    /// any number of times with the same result.
    ///
    /// # Errors
    /// The copy cannot be written, or the operating system refuses the entry.
    pub fn register(&self, running_program: &Path) -> Result<PathBuf, CliError> {
        let installed = self.installed_program();
        copy_when_different(running_program, &installed).map_err(|source| CliError::Files {
            path: installed.clone(),
            source,
        })?;
        self.login_item.enable(&installed)?;
        Ok(installed)
    }

    /// Removes the login item and every copy. The data folder's other files stay.
    ///
    /// # Errors
    /// The operating system refuses to remove the entry, or a copy cannot be
    /// deleted (Windows keeps the file of a running program).
    pub fn unregister(&self) -> Result<(), CliError> {
        self.login_item.disable()?;
        match fs::remove_dir_all(self.copies_folder()) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(CliError::Files {
                path: self.copies_folder(),
                source: error,
            }),
            _ => Ok(()),
        }
    }

    /// # Errors
    /// The operating system did not answer.
    pub fn is_registered(&self) -> Result<bool, CliError> {
        Ok(self.login_item.is_enabled()?)
    }

    fn copies_folder(&self) -> PathBuf {
        self.data_folder.join(COPIES_FOLDER)
    }
}

fn copy_when_different(source: &Path, target: &Path) -> io::Result<()> {
    if already_installed(source, target)? {
        return Ok(());
    }
    let folder = target
        .parent()
        .ok_or_else(|| io::Error::other("the copy has no folder"))?;
    fs::create_dir_all(folder)?;
    // A copy that stops halfway must never be the file that login starts.
    let partial = target.with_extension(PARTIAL_EXTENSION);
    fs::copy(source, &partial)?;
    fs::rename(&partial, target)
}

/// Copying a file onto itself would empty it, so the same file counts as installed.
fn already_installed(source: &Path, target: &Path) -> io::Result<bool> {
    if !target.try_exists()? {
        return Ok(false);
    }
    if fs::canonicalize(source)? == fs::canonicalize(target)? {
        return Ok(true);
    }
    if fs::metadata(source)?.len() != fs::metadata(target)?.len() {
        return Ok(false);
    }
    Ok(fs::read(source)? == fs::read(target)?)
}

#[cfg(test)]
#[path = "login_start.tests.rs"]
mod tests;
