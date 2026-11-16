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
/// Written by `unregister`, so that the app's start does not undo it; an
/// explicit `register` removes it.
const TURNED_OFF_MARKER: &str = "login-start-off";

/// Starts a stable copy of the server at login. The copy exists because the
/// running program may sit in a place that goes away: an `AppImage` mount, or an
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
        self.files(remove_if_present(&self.turned_off_marker()))?;
        self.install(running_program)
    }

    /// Registers as `register` does on the first start of the app and on the
    /// first start of a new version, which is when this version has no copy
    /// yet (plan D13). Nothing changes, and `None` is returned, when the copy
    /// exists or the user turned login start off with `unregister`.
    ///
    /// # Errors
    /// As `register`.
    pub fn register_at_app_start(
        &self,
        running_program: &Path,
    ) -> Result<Option<PathBuf>, CliError> {
        if path_exists(self.turned_off_marker())? || path_exists(self.installed_program())? {
            return Ok(None);
        }
        self.install(running_program).map(Some)
    }

    fn install(&self, running_program: &Path) -> Result<PathBuf, CliError> {
        let installed = self.installed_program();
        copy_when_different(running_program, &installed).map_err(|source| CliError::Files {
            path: installed.clone(),
            source,
        })?;
        self.login_item.enable(&installed)?;
        Ok(installed)
    }

    /// Removes the login item and every copy, and records that login start
    /// is off. The data folder's other files stay.
    ///
    /// # Errors
    /// The operating system refuses to remove the entry, or a copy cannot be
    /// deleted (Windows keeps the file of a running program).
    pub fn unregister(&self) -> Result<(), CliError> {
        self.login_item.disable()?;
        self.files(write_marker(self.data_folder, &self.turned_off_marker()))?;
        let copies = self.copies_folder();
        match fs::remove_dir_all(&copies) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(CliError::Files {
                path: copies,
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

    fn turned_off_marker(&self) -> PathBuf {
        self.data_folder.join(TURNED_OFF_MARKER)
    }

    fn files(&self, outcome: io::Result<()>) -> Result<(), CliError> {
        outcome.map_err(|source| CliError::Files {
            path: self.data_folder.to_path_buf(),
            source,
        })
    }
}

fn path_exists(path: PathBuf) -> Result<bool, CliError> {
    path.try_exists()
        .map_err(|source| CliError::Files { path, source })
}

fn write_marker(data_folder: &Path, marker: &Path) -> io::Result<()> {
    private_fs::create_private_dir_all(data_folder)?;
    fs::write(marker, b"")
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

fn copy_when_different(source: &Path, target: &Path) -> io::Result<()> {
    if already_installed(source, target)? {
        return Ok(());
    }
    let folder = target
        .parent()
        .ok_or_else(|| io::Error::other("the copy has no folder"))?;
    private_fs::create_private_dir_all(folder)?;
    // A copy that stops halfway must never be the file that login starts. The
    // process id keeps two `register` runs (the app's start and the button)
    // from writing into one file.
    let partial = target.with_extension(format!("{}.{PARTIAL_EXTENSION}", std::process::id()));
    remove_if_present(&partial)?;
    let copied = write_partial(source, &partial).and_then(|()| fs::rename(&partial, target));
    if copied.is_err() {
        // The copy's own error is the one to report; a leftover is replaced next time.
        let _ = fs::remove_file(&partial);
    }
    copied
}

/// `create_new` refuses a file or a link that is already at the path.
fn write_partial(source: &Path, partial: &Path) -> io::Result<()> {
    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(partial)?;
    io::copy(&mut fs::File::open(source)?, &mut writer)?;
    writer.sync_all()?;
    fs::set_permissions(partial, fs::metadata(source)?.permissions())
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
