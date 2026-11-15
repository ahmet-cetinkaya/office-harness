// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};

use crate::services::login_environment::Environment;

const DEFAULT_WINDOWS_EXTENSIONS: &str = ".COM;.EXE;.BAT;.CMD";

/// Finds the file that `program` names on the `PATH` of `environment`, and
/// returns its absolute path. A name with a folder part is checked as it is,
/// or against `working_folder` when it is relative. A `PATH` entry that is
/// not absolute is skipped: it would mean a different folder for each agent.
/// On Windows, the extensions of `PATHEXT` are tried, so `claude` finds
/// `claude.cmd`.
#[must_use]
pub fn find_program(
    program: &str,
    environment: &Environment,
    working_folder: &Path,
) -> Option<PathBuf> {
    if program.is_empty() {
        return None;
    }
    if Path::new(program).components().count() > 1 {
        let file = working_folder.join(program);
        return find_runnable(&file, environment);
    }
    let path = variable(environment, "PATH")?;
    std::env::split_paths(path)
        .filter(|folder| folder.is_absolute())
        .find_map(|folder| find_runnable(&folder.join(program), environment))
}

fn find_runnable(file: &Path, environment: &Environment) -> Option<PathBuf> {
    candidates(file, environment)
        .into_iter()
        .find(|candidate| is_runnable(candidate))
        .filter(|found| found.is_absolute())
}

/// The file itself, and on Windows the file with each extension.
fn candidates(file: &Path, environment: &Environment) -> Vec<PathBuf> {
    let mut found = vec![file.to_path_buf()];
    if cfg!(windows) {
        let extensions = variable(environment, "PATHEXT").unwrap_or(DEFAULT_WINDOWS_EXTENSIONS);
        for extension in extensions
            .split(';')
            .filter(|extension| !extension.is_empty())
        {
            let mut name = file.as_os_str().to_owned();
            name.push(extension);
            found.push(PathBuf::from(name));
        }
    }
    found
}

/// Windows variable names do not depend on case.
fn variable<'a>(environment: &'a Environment, name: &str) -> Option<&'a str> {
    if cfg!(windows) {
        environment
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    } else {
        environment.get(name).map(String::as_str)
    }
}

#[cfg(unix)]
fn is_runnable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    const ANY_EXECUTE_BIT: u32 = 0o111;
    path.metadata().is_ok_and(|metadata| {
        metadata.is_file() && metadata.permissions().mode() & ANY_EXECUTE_BIT != 0
    })
}

#[cfg(not(unix))]
fn is_runnable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
#[path = "program_lookup.tests.rs"]
mod tests;
