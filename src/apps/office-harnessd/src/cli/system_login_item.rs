// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};

#[cfg(target_os = "linux")]
use auto_launch::LinuxLaunchMode;
#[cfg(target_os = "macos")]
use auto_launch::MacOSLaunchMode;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};

use super::{LoginItem, LoginItemError};

const APP_NAME: &str = "office-harnessd";
/// Characters that the Linux unit and desktop entry would read as a variable,
/// a field code, a specifier, or an escape inside the quoted path.
#[cfg(target_os = "linux")]
const UNSAFE_PATH_CHARACTERS: [char; 6] = ['"', '\\', '$', '`', '%', '\n'];
#[cfg(target_os = "windows")]
const UNSAFE_PATH_CHARACTERS: [char; 1] = ['"'];

/// The real login item: a systemd user unit or an XDG autostart entry on
/// Linux, a `LaunchAgent` on macOS, and a `Run` registry value on Windows.
pub struct SystemLoginItem;

impl LoginItem for SystemLoginItem {
    fn enable(&self, program: &Path) -> Result<(), LoginItemError> {
        let mut entries = entries_for(program)?.into_iter();
        let preferred = entries
            .next()
            .ok_or_else(|| LoginItemError("no login item mode is available".to_owned()))?;
        // Another mode may hold an older entry; two entries would start two servers.
        for other in entries {
            remove_if_enabled(&other)?;
        }
        #[cfg(target_os = "linux")]
        systemd_restart_rule::write()?;
        preferred.enable().map_err(describe)
    }

    fn disable(&self) -> Result<(), LoginItemError> {
        let current_program = running_program()?;
        for entry in entries_for(&current_program)? {
            remove_if_enabled(&entry)?;
        }
        #[cfg(target_os = "linux")]
        systemd_restart_rule::remove()?;
        Ok(())
    }

    fn is_enabled(&self) -> Result<bool, LoginItemError> {
        let current_program = running_program()?;
        for entry in entries_for(&current_program)? {
            if entry.is_enabled().map_err(describe)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Removing and querying an entry need no real target, so the running
/// program stands in.
fn running_program() -> Result<PathBuf, LoginItemError> {
    std::env::current_exe()
        .map_err(|error| LoginItemError(format!("the running program cannot be found: {error}")))
}

fn remove_if_enabled(entry: &AutoLaunch) -> Result<(), LoginItemError> {
    if entry.is_enabled().map_err(describe)? {
        entry.disable().map_err(describe)?;
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)] // Used as a `map_err` callback.
fn describe(error: auto_launch::Error) -> LoginItemError {
    LoginItemError(error.to_string())
}

fn new_builder(program: &Path) -> Result<AutoLaunchBuilder, LoginItemError> {
    let path = program
        .to_str()
        .ok_or_else(|| LoginItemError("the program path is not valid UTF-8".to_owned()))?;
    let mut builder = AutoLaunchBuilder::new();
    builder
        .set_app_name(APP_NAME)
        .set_app_path(&command_line_path(path)?);
    Ok(builder)
}

/// The Linux unit and desktop entry and the Windows `Run` value are command
/// lines that the crate writes without quotes, so a space in the home folder
/// would split the path. A character that has a meaning inside the quotes is
/// refused rather than escaped differently for each format.
#[cfg(not(target_os = "macos"))]
fn command_line_path(path: &str) -> Result<String, LoginItemError> {
    if let Some(unsafe_character) = path.chars().find(|c| UNSAFE_PATH_CHARACTERS.contains(c)) {
        return Err(LoginItemError(format!(
            "the program path contains {unsafe_character:?}, which a login item cannot hold: {path}"
        )));
    }
    Ok(format!("\"{path}\""))
}

/// The macOS property list holds the path as one argument, as is.
#[cfg(target_os = "macos")]
#[allow(clippy::unnecessary_wraps)] // Same signature as on the other systems.
fn command_line_path(path: &str) -> Result<String, LoginItemError> {
    Ok(path.to_owned())
}

/// The modes that can hold an entry for this user, the preferred one first.
/// Systemd comes first when `systemctl --user` answers; XDG autostart needs
/// the desktop session, so it is the fallback (research R4).
#[cfg(target_os = "linux")]
fn entries_for(program: &Path) -> Result<Vec<AutoLaunch>, LoginItemError> {
    let mut modes = Vec::new();
    if systemd_user_answers() {
        modes.push(LinuxLaunchMode::Systemd);
    }
    modes.push(LinuxLaunchMode::XdgAutostart);
    modes
        .into_iter()
        .map(|mode| {
            let mut builder = new_builder(program)?;
            builder.set_linux_launch_mode(mode);
            builder.build().map_err(describe)
        })
        .collect()
}

/// A drop-in next to the crate's unit: the unit restarts the server after a
/// failure, but a second server that exits because one already runs must not
/// be started again every few seconds.
#[cfg(target_os = "linux")]
mod systemd_restart_rule {
    use std::fs;
    use std::io;
    use std::path::PathBuf;

    use directories::BaseDirs;

    use super::{APP_NAME, LoginItemError};
    use crate::bootstrap::ALREADY_RUNNING_EXIT_CODE;

    const RULE_FILE: &str = "office-harness.conf";

    pub fn content() -> String {
        format!("[Service]\nRestartPreventExitStatus={ALREADY_RUNNING_EXIT_CODE}\n")
    }

    /// The same folder as the crate's unit: `~/.config/systemd/user`.
    fn rule_folder() -> Result<PathBuf, LoginItemError> {
        let home = BaseDirs::new()
            .ok_or_else(|| LoginItemError("the home folder cannot be found".to_owned()))?;
        Ok(home
            .home_dir()
            .join(".config/systemd/user")
            .join(format!("{APP_NAME}.service.d")))
    }

    pub fn write() -> Result<(), LoginItemError> {
        let folder = rule_folder()?;
        fs::create_dir_all(&folder)
            .and_then(|()| fs::write(folder.join(RULE_FILE), content()))
            .map_err(|error| LoginItemError(format!("the systemd restart rule: {error}")))
    }

    pub fn remove() -> Result<(), LoginItemError> {
        let folder = rule_folder()?;
        match fs::remove_dir_all(&folder) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                Err(LoginItemError(format!("the systemd restart rule: {error}")))
            }
            _ => Ok(()),
        }
    }
}

#[cfg(target_os = "linux")]
fn systemd_user_answers() -> bool {
    Command::new("systemctl")
        .args(["--user", "show-environment"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// A `LaunchAgent` avoids the `AppleScript` prompt of the other macOS mode; on
/// Windows the entry is a value under the current user's `Run` key.
#[cfg(not(target_os = "linux"))]
fn entries_for(program: &Path) -> Result<Vec<AutoLaunch>, LoginItemError> {
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut builder = new_builder(program)?;
    #[cfg(target_os = "macos")]
    builder.set_macos_launch_mode(MacOSLaunchMode::LaunchAgent);
    Ok(vec![builder.build().map_err(describe)?])
}

#[cfg(test)]
#[path = "system_login_item.tests.rs"]
mod tests;
