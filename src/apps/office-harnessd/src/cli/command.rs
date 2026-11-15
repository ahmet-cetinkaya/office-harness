// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsString;

const REGISTER: &str = "register";
const UNREGISTER: &str = "unregister";
const STATUS: &str = "status";

/// What the program does instead of serving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Register,
    Unregister,
    Status,
}

/// Splits the arguments (without the program name) into a subcommand and the
/// arguments after it. `None` means the program serves.
#[must_use]
pub fn split_command(arguments: &[OsString]) -> Option<(Command, &[OsString])> {
    let (first, rest) = arguments.split_first()?;
    let command = if first == REGISTER {
        Command::Register
    } else if first == UNREGISTER {
        Command::Unregister
    } else if first == STATUS {
        Command::Status
    } else {
        return None;
    };
    Some((command, rest))
}

#[cfg(test)]
#[path = "command.tests.rs"]
mod tests;
