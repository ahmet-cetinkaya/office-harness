// SPDX-License-Identifier: AGPL-3.0-or-later

//! The folder side of the file view (0016 D3 to D5): every path is resolved,
//! links included, and read only when the resolved path is inside the root.

use std::fs::{self, File, FileType};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_application::common::interfaces::FileViewError;
use agent_application::common::models::{
    ContentState, EntryKind, FileContent, FolderEntry, FolderListing, LinkTarget,
};
use agent_domain::services::{
    MAX_CONTENT_BYTES, MAX_LIST_ENTRIES, RelativePath, is_binary, is_secret_looking,
};
use kernel::Timestamp;

const GIT_FOLDER: &str = ".git";
const OUTSIDE_REFUSAL: &str = "the path is outside the agent's working folder";
const NOT_A_FILE: &str = "not a regular file";
const NOT_A_FOLDER: &str = "not a folder";
const FOLDER_FAILURE: &str = "the folder cannot be read";
const FILE_FAILURE: &str = "the file cannot be read";
/// One byte more than the limit tells a file over it from one at it, and
/// still caps a file that grows after the check.
const READ_LIMIT_BYTES: u64 = MAX_CONTENT_BYTES as u64 + 1;

/// The root with every link in it resolved.
///
/// # Errors
/// `Failed` when the root cannot be resolved, for example when it does not
/// exist.
pub fn canonical_root(root: &Path) -> Result<PathBuf, FileViewError> {
    root.canonicalize()
        .map_err(|error| failure(FOLDER_FAILURE, &error))
}

/// The resolved path of `path` below `root`, which must be canonical. A path
/// that does not exist is resolved through its deepest existing ancestor:
/// the parts after it cannot be links.
///
/// # Errors
/// `Refused` when the resolved path is outside the root or holds a part
/// named `.git`.
pub fn contained_path(root: &Path, path: &RelativePath) -> Result<PathBuf, FileViewError> {
    let joined = path
        .parts()
        .iter()
        .fold(root.to_path_buf(), |joined, part| joined.join(part));
    let resolved = resolve_through_existing_ancestor(&joined);
    if is_inside(root, &resolved) {
        return Ok(resolved);
    }
    Err(FileViewError::Refused(OUTSIDE_REFUSAL.to_owned()))
}

/// True when the requested name or the resolved name (a link named `notes`
/// to `.env`) is secret-looking.
#[must_use]
pub fn has_secret_name(path: &RelativePath, resolved: &Path) -> bool {
    path.name().is_some_and(is_secret_looking)
        || resolved
            .file_name()
            .is_some_and(|name| is_secret_looking(&name.to_string_lossy()))
}

/// The folder `path` lists: folders first, then by name, at most 1,000.
///
/// # Errors
/// `Refused` outside the root, `Invalid` for a file, `Failed` when the
/// folder cannot be read.
pub fn list_folder(root: &Path, path: &RelativePath) -> Result<FolderListing, FileViewError> {
    let root = canonical_root(root)?;
    let folder = contained_path(&root, path)?;
    let metadata = fs::metadata(&folder).map_err(|error| failure(FOLDER_FAILURE, &error))?;
    if !metadata.is_dir() {
        return Err(FileViewError::Invalid(NOT_A_FOLDER.to_owned()));
    }
    let mut found = visible_entries(&folder)?;
    found.sort_by(|left, right| {
        (!left.is_folder())
            .cmp(&!right.is_folder())
            .then_with(|| left.name.cmp(&right.name))
    });
    let more = found.len().saturating_sub(MAX_LIST_ENTRIES);
    found.truncate(MAX_LIST_ENTRIES);
    Ok(FolderListing {
        root: root.to_string_lossy().into_owned(),
        path: path.to_text(),
        entries: found.iter().map(|entry| entry.describe(&root)).collect(),
        more,
        read_at: now(),
    })
}

/// The content of one file, at most 512 KiB of it. A secret-looking file is
/// not opened unless `reveal` is set.
///
/// # Errors
/// `Refused` outside the root, `Invalid` for an entry that is not a regular
/// file, `Failed` when the file cannot be read.
pub fn read_file(
    root: &Path,
    path: &RelativePath,
    reveal: bool,
) -> Result<FileContent, FileViewError> {
    let root = canonical_root(root)?;
    let resolved = contained_path(&root, path)?;
    let content = |state, size, bytes| FileContent {
        path: path.to_text(),
        state,
        size,
        bytes,
        read_at: now(),
    };
    if !reveal && has_secret_name(path, &resolved) {
        let size = fs::metadata(&resolved).map_or(0, |metadata| metadata.len());
        return Ok(content(ContentState::Secret, size, None));
    }
    let (bytes, size) = read_limited(&resolved)?;
    if bytes.len() > MAX_CONTENT_BYTES {
        return Ok(content(ContentState::TooLarge, size, None));
    }
    if is_binary(&bytes) {
        return Ok(content(ContentState::Binary, size, None));
    }
    Ok(content(ContentState::Text, size, Some(bytes)))
}

/// The time of a read, for the `read_at` of an answer.
pub(crate) fn now() -> Timestamp {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Timestamp::from_unix_millis(i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
}

/// What a read of the disk failed on, in words a user reads.
pub(crate) fn failure(action: &str, error: &io::Error) -> FileViewError {
    let reason = match error.kind() {
        io::ErrorKind::NotFound => "it does not exist".to_owned(),
        io::ErrorKind::PermissionDenied => "access was denied".to_owned(),
        _ => error.to_string(),
    };
    FileViewError::Failed(format!("{action}: {reason}"))
}

/// The canonical form of `joined`, built from its deepest ancestor that
/// resolves; a missing or broken part cannot lead anywhere else.
fn resolve_through_existing_ancestor(joined: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut existing = joined;
    let mut resolved = loop {
        if let Ok(canonical) = existing.canonicalize() {
            break canonical;
        }
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                missing.push(name);
                existing = parent;
            }
            _ => return joined.to_path_buf(),
        }
    };
    resolved.extend(missing.iter().rev());
    resolved
}

/// True when `resolved` is the root or below it and holds no `.git` part
/// below the root: the git data is never listed or read.
fn is_inside(root: &Path, resolved: &Path) -> bool {
    resolved.strip_prefix(root).is_ok_and(|below| {
        below
            .components()
            .all(|part| !part.as_os_str().eq_ignore_ascii_case(GIT_FOLDER))
    })
}

/// One entry of a folder before its details are read.
struct FoundEntry {
    name: String,
    file_type: FileType,
    path: PathBuf,
}

impl FoundEntry {
    fn is_folder(&self) -> bool {
        self.file_type.is_dir()
    }

    fn describe(&self, root: &Path) -> FolderEntry {
        let kind = if self.file_type.is_symlink() {
            EntryKind::Link
        } else if self.file_type.is_dir() {
            EntryKind::Folder
        } else if self.file_type.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        FolderEntry {
            name: self.name.clone(),
            kind,
            size: (kind == EntryKind::File)
                .then(|| fs::symlink_metadata(&self.path).map(|metadata| metadata.len()))
                .and_then(Result::ok),
            secret: is_secret_looking(&self.name),
            target: (kind == EntryKind::Link).then(|| link_target(root, &self.path)),
        }
    }
}

/// The entries of `folder` without the git data.
fn visible_entries(folder: &Path) -> Result<Vec<FoundEntry>, FileViewError> {
    let reader = fs::read_dir(folder).map_err(|error| failure(FOLDER_FAILURE, &error))?;
    let mut found = Vec::new();
    // An entry that vanishes while the folder is read is no longer there.
    for entry in reader.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !name.eq_ignore_ascii_case(GIT_FOLDER) {
            found.push(FoundEntry {
                name,
                file_type,
                path: entry.path(),
            });
        }
    }
    Ok(found)
}

fn link_target(root: &Path, link: &Path) -> LinkTarget {
    let Ok(resolved) = link.canonicalize() else {
        return LinkTarget::Broken;
    };
    if !is_inside(root, &resolved) {
        return LinkTarget::Outside;
    }
    match fs::metadata(&resolved) {
        Ok(metadata) if metadata.is_dir() => LinkTarget::Folder,
        Ok(metadata) if metadata.is_file() => LinkTarget::File,
        Ok(_) => LinkTarget::Other,
        Err(_) => LinkTarget::Broken,
    }
}

/// At most one byte over the limit of an opened regular file, and the size
/// the file has.
fn read_limited(resolved: &Path) -> Result<(Vec<u8>, u64), FileViewError> {
    let file = open_without_blocking(resolved).map_err(|error| failure(FILE_FAILURE, &error))?;
    let metadata = file
        .metadata()
        .map_err(|error| failure(FILE_FAILURE, &error))?;
    if !metadata.is_file() {
        return Err(FileViewError::Invalid(NOT_A_FILE.to_owned()));
    }
    let mut bytes = Vec::new();
    file.take(READ_LIMIT_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|error| failure(FILE_FAILURE, &error))?;
    let read = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let size = metadata.len().max(read);
    Ok((bytes, size))
}

/// A pipe swapped in after the check must not block the open, and a link
/// swapped in for the last part must not be followed.
#[cfg(unix)]
fn open_without_blocking(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    use nix::fcntl::OFlag;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags((OFlag::O_NONBLOCK | OFlag::O_NOFOLLOW).bits())
        .open(path)
}

#[cfg(not(unix))]
fn open_without_blocking(path: &Path) -> io::Result<File> {
    File::open(path)
}

#[cfg(test)]
#[path = "folder_reader.tests.rs"]
mod tests;
