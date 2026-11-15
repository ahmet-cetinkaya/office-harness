// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::{self, DirBuilder, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

const FOLDER_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;

pub fn create_dir_all_private(folder: &Path) -> io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(FOLDER_MODE)
        .create(folder)
}

pub fn apply_creation_mode(options: &mut OpenOptions) {
    options.mode(FILE_MODE);
}

pub fn restrict_folder(folder: &Path) -> io::Result<()> {
    set_mode(folder, FOLDER_MODE)
}

pub fn restrict_file(path: &Path) -> io::Result<()> {
    set_mode(path, FILE_MODE)
}

fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    let current = fs::metadata(path)?.permissions().mode() & 0o777;
    if current == mode {
        return Ok(());
    }
    fs::set_permissions(path, Permissions::from_mode(mode))
}
