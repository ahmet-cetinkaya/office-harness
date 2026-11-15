// SPDX-License-Identifier: AGPL-3.0-or-later

//! Owner-only access on Windows through the system `icacls` tool: it breaks
//! inheritance, removes every remaining entry, and grants full control to the
//! current user and to SYSTEM only. The crate stays free of `unsafe` code this
//! way.

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::fs::{self, OpenOptions};
#[cfg(windows)]
use std::io;
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;

const SYSTEM_SID: &str = "S-1-5-18";
const FOLDER_INHERITANCE: &str = "(OI)(CI)";
const FILE_INHERITANCE: &str = "";
const FULL_CONTROL: &str = "F";

/// The number of entries an owner-only access list holds: the user and SYSTEM.
#[cfg(windows)]
const OWNER_ONLY_ENTRIES: usize = 2;

/// The `icacls` arguments that replace the access list of `path`.
fn restrict_arguments(path: &str, user_sid: &str, inheritance: &str) -> Vec<String> {
    let grant = |sid: &str| format!("*{sid}:{inheritance}{FULL_CONTROL}");
    vec![
        path.to_owned(),
        "/inheritance:r".to_owned(),
        "/grant:r".to_owned(),
        grant(user_sid),
        "/grant:r".to_owned(),
        grant(SYSTEM_SID),
    ]
}

/// The account names in the output of `icacls <path>`, one per access entry.
/// Names are read as `icacls` prints them, so they may be localized.
fn access_list_principals(icacls_output: &str, path: &str) -> Vec<String> {
    icacls_output
        .lines()
        .map(|line| line.strip_prefix(path).unwrap_or(line).trim())
        .filter_map(|line| {
            line.split_once(":(")
                .map(|(principal, _)| principal.to_owned())
        })
        .collect()
}

#[cfg(windows)]
pub fn create_dir_all_private(folder: &Path) -> io::Result<()> {
    fs::create_dir_all(folder)
}

#[cfg(windows)]
pub fn apply_creation_mode(_options: &mut OpenOptions) {}

#[cfg(windows)]
pub fn restrict_folder(folder: &Path) -> io::Result<()> {
    restrict(folder, FOLDER_INHERITANCE)
}

#[cfg(windows)]
pub fn restrict_file(path: &Path) -> io::Result<()> {
    restrict(path, FILE_INHERITANCE)
}

/// Replaces the access list of `path` with the current user and SYSTEM only.
/// `/grant:r` replaces only the entries it names, so an explicit entry that a
/// new file or folder gets from the creating token (for example
/// `BUILTIN\Administrators` under an elevated account) is removed first. The
/// result is read back, and anything other than the two entries is an error.
#[cfg(windows)]
fn restrict(path: &Path, inheritance: &str) -> io::Result<()> {
    let path_text = path.to_string_lossy();
    run_system_tool("icacls.exe", &[path_text.as_ref(), "/inheritance:r"])?;
    remove_every_entry(&path_text)?;
    let arguments = restrict_arguments(&path_text, &current_user_sid()?, inheritance);
    run_system_tool("icacls.exe", &arguments)?;
    ensure_owner_only(&path_text)
}

#[cfg(windows)]
fn remove_every_entry(path: &str) -> io::Result<()> {
    let listing = run_system_tool("icacls.exe", &[path])?;
    for principal in access_list_principals(&listing, path) {
        run_system_tool("icacls.exe", &[path, "/remove:g", principal.as_str()])?;
    }
    Ok(())
}

#[cfg(windows)]
fn ensure_owner_only(path: &str) -> io::Result<()> {
    let listing = run_system_tool("icacls.exe", &[path])?;
    let entries = access_list_principals(&listing, path).len();
    if entries != OWNER_ONLY_ENTRIES || listing.contains("(I)") {
        return Err(io::Error::other(format!(
            "the access list of {path} is not owner-only after the change"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn current_user_sid() -> io::Result<String> {
    let output = run_system_tool("whoami.exe", &["/user", "/fo", "csv", "/nh"])?;
    output
        .trim()
        .rsplit(',')
        .next()
        .map(|sid| sid.trim_matches('"').to_owned())
        .filter(|sid| sid.starts_with("S-"))
        .ok_or_else(|| io::Error::other("whoami did not report the current user SID"))
}

/// Runs a tool from the Windows system folder, so that a tool of the same name
/// earlier on `PATH` cannot take its place.
#[cfg(windows)]
fn run_system_tool<S: AsRef<OsStr>>(tool: &str, arguments: &[S]) -> io::Result<String> {
    let system_root =
        std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("SystemRoot is not set"))?;
    let tool_path: PathBuf = [system_root.as_os_str(), "System32".as_ref(), tool.as_ref()]
        .iter()
        .collect();
    let output = Command::new(tool_path).args(arguments).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "{tool} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
#[path = "windows_access_list.tests.rs"]
mod tests;
