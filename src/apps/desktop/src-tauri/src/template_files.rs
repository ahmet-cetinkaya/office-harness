// SPDX-License-Identifier: AGPL-3.0-or-later

//! The two file actions of the templates view (0009 D6). They work on a path
//! the user picked in a system dialog and never on a path from the webview.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use thiserror::Error;

/// The largest import file; the server refuses a longer text too.
pub const MAX_IMPORT_BYTES: u64 = 256 * 1024;

#[derive(Debug, Error)]
pub enum TemplateFileError {
    #[error("the file was not read: it is not a regular file")]
    NotRegularFile,
    #[error("the file was not read: it is larger than 256 KiB")]
    TooLarge,
    #[error("the file was not read: it is not text")]
    NotText,
    #[error("the file was not read: {0}")]
    Read(#[source] io::Error),
    #[error("the file was not written: {0}")]
    Write(#[source] io::Error),
}

/// Reads the picked file. The size cap counts the bytes read, not the size
/// that the file system reports, and a FIFO or a device is refused before it
/// is opened, because opening a FIFO can wait forever.
pub fn read_import_file(path: &Path) -> Result<String, TemplateFileError> {
    if !fs::metadata(path)
        .map_err(TemplateFileError::Read)?
        .is_file()
    {
        return Err(TemplateFileError::NotRegularFile);
    }
    let file = File::open(path).map_err(TemplateFileError::Read)?;
    if !file.metadata().map_err(TemplateFileError::Read)?.is_file() {
        return Err(TemplateFileError::NotRegularFile);
    }
    let mut bytes = Vec::new();
    file.take(MAX_IMPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(TemplateFileError::Read)?;
    if bytes.len() as u64 > MAX_IMPORT_BYTES {
        return Err(TemplateFileError::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| TemplateFileError::NotText)
}

pub fn write_export_file(path: &Path, contents: &str) -> Result<(), TemplateFileError> {
    fs::write(path, contents).map_err(TemplateFileError::Write)
}

#[cfg(test)]
#[path = "template_files.tests.rs"]
mod tests;
