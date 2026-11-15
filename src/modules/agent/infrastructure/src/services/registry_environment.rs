// SPDX-License-Identifier: AGPL-3.0-or-later

//! The Windows environment: the machine and the user variables of the
//! registry, read with the system `reg` tool so the crate needs no `unsafe`
//! code. Expandable values (`%NAME%`) are expanded here.

use crate::services::login_environment::Environment;

#[cfg(windows)]
const MACHINE_KEY: &str = r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
#[cfg(windows)]
const USER_KEY: &str = r"HKCU\Environment";
const COLUMN_SEPARATOR: &str = "    ";
const VALUE_TYPES: [&str; 2] = ["REG_SZ", "REG_EXPAND_SZ"];
const PATH_NAME: &str = "PATH";

/// Reads both keys and lays them over `base`. `PATH` is the machine entries
/// followed by the user entries. `None` when the machine key cannot be read.
#[cfg(windows)]
pub fn read_registry_environment(base: &Environment) -> Option<Environment> {
    let machine = parse_registry_output(&query_key(MACHINE_KEY)?);
    let user = parse_registry_output(&query_key(USER_KEY).unwrap_or_default());
    Some(layer_registry_environment(base, &machine, &user))
}

#[cfg(windows)]
fn query_key(key: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = Command::new("reg")
        .args(["query", key])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parses the lines of `reg query`: `    Name    REG_SZ    Value`.
pub fn parse_registry_output(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| {
            let mut columns = line.trim_start().splitn(3, COLUMN_SEPARATOR);
            let (name, value_type, value) = (columns.next()?, columns.next()?, columns.next()?);
            VALUE_TYPES
                .contains(&value_type.trim())
                .then(|| (name.trim().to_owned(), value.trim_end().to_owned()))
        })
        .collect()
}

pub fn layer_registry_environment(
    base: &Environment,
    machine: &[(String, String)],
    user: &[(String, String)],
) -> Environment {
    let mut environment = base.clone();
    let machine_path = value_of(machine, PATH_NAME);
    let user_path = value_of(user, PATH_NAME);
    for (name, value) in machine.iter().chain(user) {
        if !name.eq_ignore_ascii_case(PATH_NAME) {
            environment.insert(name.clone(), expand(value, &environment));
        }
    }
    let joined: Vec<String> = [machine_path, user_path]
        .into_iter()
        .flatten()
        .map(|path| expand(&path, &environment))
        .collect();
    if !joined.is_empty() {
        environment.retain(|name, _| !name.eq_ignore_ascii_case(PATH_NAME));
        environment.insert(PATH_NAME.to_owned(), joined.join(";"));
    }
    environment
}

fn value_of(entries: &[(String, String)], name: &str) -> Option<String> {
    entries
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}

/// Replaces each `%NAME%` that `environment` knows, ignoring case. An unknown
/// name stays as written.
fn expand(value: &str, environment: &Environment) -> String {
    let mut expanded = String::new();
    let mut rest = value;
    while let Some(open) = rest.find('%') {
        let after_open = &rest[open + 1..];
        let Some(length) = after_open.find('%') else {
            break;
        };
        let name = &after_open[..length];
        let known = environment
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, replacement)| replacement.as_str());
        expanded.push_str(&rest[..open]);
        if let Some(replacement) = known {
            expanded.push_str(replacement);
        } else {
            expanded.push('%');
            expanded.push_str(name);
            expanded.push('%');
        }
        rest = &after_open[length + 1..];
    }
    expanded.push_str(rest);
    expanded
}

#[cfg(test)]
#[path = "registry_environment.tests.rs"]
mod tests;
