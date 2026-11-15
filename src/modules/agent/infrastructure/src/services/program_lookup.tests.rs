// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;

use tempfile::TempDir;

use super::*;

fn path_environment(folder: &Path) -> Environment {
    Environment::from([("PATH".to_owned(), folder.to_string_lossy().into_owned())])
}

#[cfg(unix)]
fn write_tool(folder: &Path, name: &str, mode: u32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = folder.join(name);
    fs::write(&path, "#!/bin/sh\n").expect("write tool");
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("set mode");
    path
}

#[cfg(unix)]
#[test]
fn finds_an_executable_on_the_path() {
    let folder = TempDir::new().expect("temp folder");
    let tool = write_tool(folder.path(), "claude", 0o755);
    assert_eq!(
        find_program("claude", &path_environment(folder.path()), folder.path()),
        Some(tool)
    );
}

#[cfg(unix)]
#[test]
fn skips_a_file_that_is_not_executable() {
    let folder = TempDir::new().expect("temp folder");
    write_tool(folder.path(), "claude", 0o644);
    assert_eq!(
        find_program("claude", &path_environment(folder.path()), folder.path()),
        None
    );
}

#[test]
fn a_missing_program_or_path_finds_nothing() {
    let folder = TempDir::new().expect("temp folder");
    assert_eq!(
        find_program(
            "nothing-here",
            &path_environment(folder.path()),
            folder.path()
        ),
        None
    );
    assert_eq!(
        find_program("claude", &Environment::new(), folder.path()),
        None
    );
    assert_eq!(
        find_program("", &path_environment(folder.path()), folder.path()),
        None
    );
}

#[cfg(windows)]
#[test]
fn finds_a_cmd_shim_through_pathext() {
    let folder = TempDir::new().expect("temp folder");
    let shim = folder.path().join("claude.cmd");
    fs::write(&shim, "@echo off\r\n").expect("write shim");
    // Windows file names ignore case, and the extension comes from `PATHEXT`
    // in its own case (`.CMD`), so compare the paths without case.
    let found = find_program("claude", &path_environment(folder.path()), folder.path())
        .expect("the shim is found");
    assert!(
        found
            .to_string_lossy()
            .eq_ignore_ascii_case(&shim.to_string_lossy()),
        "found {found:?}, expected {shim:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_relative_path_entry_is_skipped() {
    let folder = TempDir::new().expect("temp folder");
    write_tool(folder.path(), "claude", 0o755);
    let environment = Environment::from([("PATH".to_owned(), ".".to_owned())]);
    assert_eq!(find_program("claude", &environment, folder.path()), None);
}

#[cfg(unix)]
#[test]
fn a_command_with_a_folder_resolves_against_the_working_folder() {
    let folder = TempDir::new().expect("temp folder");
    fs::create_dir(folder.path().join("bin")).expect("folder");
    let tool = write_tool(&folder.path().join("bin"), "claude", 0o755);
    let found = find_program("bin/claude", &Environment::new(), folder.path());
    assert_eq!(found, Some(tool));
    assert!(found.expect("found").is_absolute());
}
