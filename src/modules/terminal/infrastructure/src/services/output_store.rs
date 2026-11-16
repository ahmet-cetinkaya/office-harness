// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use private_fs::{create_private_dir_all, open_private_file, replace_private_file};
use terminal_domain::value_objects::{OutputKey, OutputLimits};

const NEWLINE: u8 = b'\n';
/// A trim leaves room below each limit, so the next trim is not due at the
/// next write.
const KEEP_PERCENT: usize = 90;

/// What one append did to the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendOutcome {
    pub dropped_oldest: bool,
}

/// The recent output of one session, in one owner-only file that is capped by
/// lines and by bytes. When a cap is passed, the oldest output goes first.
pub struct OutputStore {
    path: PathBuf,
    file: File,
    limits: OutputLimits,
    byte_count: usize,
    line_count: usize,
}

impl OutputStore {
    /// Starts an empty file for a new session.
    ///
    /// # Errors
    /// Returns the I/O error of the folder or file creation.
    pub fn create(folder: &Path, key: &OutputKey, limits: OutputLimits) -> io::Result<Self> {
        create_private_dir_all(folder)?;
        let path = file_path(folder, key);
        replace_private_file(&path, b"")?;
        let file = open_for_append(&path)?;
        Ok(Self {
            path,
            file,
            limits,
            byte_count: 0,
            line_count: 0,
        })
    }

    /// The saved output of a session, or `None` when no file exists.
    ///
    /// # Errors
    /// Returns an I/O error other than a missing file.
    pub fn read_saved(folder: &Path, key: &OutputKey) -> io::Result<Option<Vec<u8>>> {
        match fs::read(file_path(folder, key)) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// # Errors
    /// Returns the I/O error of the write or of the trim.
    pub fn append(&mut self, bytes: &[u8]) -> io::Result<AppendOutcome> {
        self.file.write_all(bytes)?;
        self.byte_count += bytes.len();
        self.line_count += count_newlines(bytes);
        let over_limit =
            self.byte_count > self.limits.max_bytes() || self.line_count > self.limits.max_lines();
        if over_limit {
            self.trim()?;
        }
        Ok(AppendOutcome {
            dropped_oldest: over_limit,
        })
    }

    fn trim(&mut self) -> io::Result<()> {
        let contents = fs::read(&self.path)?;
        let kept = newest_part(
            &contents,
            self.limits.max_lines() * KEEP_PERCENT / 100,
            self.limits.max_bytes() * KEEP_PERCENT / 100,
        );
        replace_private_file(&self.path, kept)?;
        // The replacement is a new file, so the open handle points at the old one.
        self.file = open_for_append(&self.path)?;
        self.byte_count = kept.len();
        self.line_count = count_newlines(kept);
        Ok(())
    }
}

fn file_path(folder: &Path, key: &OutputKey) -> PathBuf {
    folder.join(format!("{key}.log"))
}

fn open_for_append(path: &Path) -> io::Result<File> {
    open_private_file(path, OpenOptions::new().append(true))
}

#[allow(
    clippy::naive_bytecount,
    reason = "one pass over a capped buffer; a counting crate would add a dependency for no gain"
)]
fn count_newlines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&byte| byte == NEWLINE).count()
}

/// The newest part of `contents` within both limits. A cut for the byte limit
/// moves forward to the next line start when the part has one.
fn newest_part(contents: &[u8], max_lines: usize, max_bytes: usize) -> &[u8] {
    let mut start = contents.len().saturating_sub(max_bytes);
    if start > 0
        && let Some(offset) = contents[start..].iter().position(|&byte| byte == NEWLINE)
    {
        start += offset + 1;
    }
    let part = &contents[start..];
    let excess_lines = count_newlines(part).saturating_sub(max_lines);
    if excess_lines == 0 {
        return part;
    }
    let first_kept = part
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == NEWLINE)
        .nth(excess_lines - 1)
        .map_or(0, |(index, _)| index + 1);
    &part[first_kept..]
}

#[cfg(test)]
#[path = "output_store.tests.rs"]
mod tests;
