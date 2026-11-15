// SPDX-License-Identifier: AGPL-3.0-or-later

//! Owner-only access on Windows: one `SetNamedSecurityInfoW` call replaces the
//! access list with a protected one that holds full control for the current
//! user and for SYSTEM only. The list is built from SIDs, so it does not depend
//! on the language of the system or on the characters of the path. This module
//! calls Win32 functions, so it is the one place in the crate that allows
//! `unsafe` code; each `unsafe` block says why it is sound.
#![allow(unsafe_code)]

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use std::fs::{self, OpenOptions};
#[cfg(windows)]
use std::io;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use std::ptr;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, LocalFree};
#[cfg(windows)]
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1, SE_FILE_OBJECT,
    SetNamedSecurityInfoW,
};
#[cfg(windows)]
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, PROTECTED_DACL_SECURITY_INFORMATION,
};

const SYSTEM_ALIAS: &str = "SY";
const FULL_CONTROL: &str = "FA";
const FOLDER_ACE_FLAGS: &str = "OICI";
const FILE_ACE_FLAGS: &str = "";

/// The SDDL of a protected access list that allows full control to `user_sid`
/// and to SYSTEM and nothing else. `ace_flags` say whether children inherit.
fn owner_only_sddl(user_sid: &str, ace_flags: &str) -> String {
    let allow = |trustee: &str| format!("(A;{ace_flags};{FULL_CONTROL};;;{trustee})");
    format!("D:P{}{}", allow(user_sid), allow(SYSTEM_ALIAS))
}

/// A block that the system allocated and that `LocalFree` must release.
#[cfg(windows)]
struct LocalAllocation(*mut c_void);

#[cfg(windows)]
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: the pointer came from a Win32 call that documents `LocalFree`
        // as the release function, and this guard is its only owner.
        unsafe { LocalFree(self.0) };
    }
}

#[cfg(windows)]
pub fn create_dir_all_private(folder: &Path) -> io::Result<()> {
    fs::create_dir_all(folder)
}

#[cfg(windows)]
pub fn apply_creation_mode(_options: &mut OpenOptions) {}

#[cfg(windows)]
pub fn restrict_folder(folder: &Path) -> io::Result<()> {
    restrict(folder, FOLDER_ACE_FLAGS)
}

#[cfg(windows)]
pub fn restrict_file(path: &Path) -> io::Result<()> {
    restrict(path, FILE_ACE_FLAGS)
}

/// Replaces the whole access list of `path` in one call. The new list is
/// protected, so it takes nothing from the parent, and it holds no other entry.
/// The owner is left as it is: setting it needs `WRITE_OWNER`, which the old
/// list may not grant, and the owner has no access through ownership alone
/// beyond reading and changing the list.
#[cfg(windows)]
fn restrict(path: &Path, ace_flags: &str) -> io::Result<()> {
    let sddl = owner_only_sddl(&current_user_sid()?, ace_flags);
    let descriptor = parse_sddl(&sddl)?;
    let dacl = dacl_of(&descriptor)?;
    let wide_path = to_wide(path.as_os_str());
    // SAFETY: `wide_path` is NUL-terminated and outlives the call; `dacl` points
    // into `descriptor`, which is alive until the end of this function; the
    // owner, group and SACL arguments are null and are not selected by the flags.
    let code = unsafe {
        SetNamedSecurityInfoW(
            wide_path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            dacl,
            ptr::null(),
        )
    };
    if code != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(code.cast_signed()));
    }
    Ok(())
}

/// The NUL-terminated UTF-16 form of `text`, as the wide Win32 functions take.
#[cfg(windows)]
fn to_wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn parse_sddl(sddl: &str) -> io::Result<LocalAllocation> {
    let wide_sddl = to_wide(sddl.as_ref());
    let mut descriptor: *mut c_void = ptr::null_mut();
    // SAFETY: `wide_sddl` is NUL-terminated and outlives the call, and
    // `descriptor` is a valid out pointer. The system allocates the descriptor
    // and the guard releases it.
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide_sddl.as_ptr(),
            SDDL_REVISION_1,
            &raw mut descriptor,
            ptr::null_mut(),
        )
    };
    if converted == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(LocalAllocation(descriptor))
}

/// The access list inside `descriptor`; it is valid while `descriptor` lives.
#[cfg(windows)]
fn dacl_of(descriptor: &LocalAllocation) -> io::Result<*mut ACL> {
    let mut present = 0;
    let mut defaulted = 0;
    let mut dacl: *mut ACL = ptr::null_mut();
    // SAFETY: `descriptor` holds a valid security descriptor, and the three
    // out pointers refer to live locals.
    let found = unsafe {
        GetSecurityDescriptorDacl(
            descriptor.0,
            &raw mut present,
            &raw mut dacl,
            &raw mut defaulted,
        )
    };
    if found == 0 {
        return Err(io::Error::last_os_error());
    }
    if present == 0 || dacl.is_null() {
        return Err(io::Error::other(
            "the security descriptor has no access list",
        ));
    }
    Ok(dacl)
}

/// The SID text of the current user, as `whoami` prints it. SIDs are ASCII, so
/// the code page of the pipe does not change them.
#[cfg(windows)]
fn current_user_sid() -> io::Result<String> {
    let system_root =
        std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("SystemRoot is not set"))?;
    // The tool comes from the system folder, so that a tool of the same name
    // earlier on `PATH` cannot take its place.
    let tool: PathBuf = [
        system_root.as_os_str(),
        "System32".as_ref(),
        "whoami.exe".as_ref(),
    ]
    .iter()
    .collect();
    let output = Command::new(tool)
        .args(["/user", "/fo", "csv", "/nh"])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other("whoami failed"));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .rsplit(',')
        .next()
        .map(|sid| sid.trim_matches('"').to_owned())
        .filter(|sid| sid.starts_with("S-"))
        .ok_or_else(|| io::Error::other("whoami did not report the current user SID"))
}

#[cfg(test)]
#[path = "windows_access_list.tests.rs"]
mod tests;
