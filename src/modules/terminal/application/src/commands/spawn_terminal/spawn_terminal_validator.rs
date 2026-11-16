// SPDX-License-Identifier: AGPL-3.0-or-later

use terminal_domain::errors::TerminalError;
use terminal_domain::services::{
    first_unsafe_batch_argument, first_unsafe_batch_character, is_batch_script,
};

use crate::common::models::SpawnTerminalRequest;

/// # Errors
/// Names the field at fault: an empty program, a relative working folder, an
/// empty variable name, or an argument that a batch script cannot receive.
pub fn validate_spawn_terminal(request: &SpawnTerminalRequest) -> Result<(), TerminalError> {
    if request.program.trim().is_empty() {
        return Err(TerminalError::Empty { field: "program" });
    }
    if !request.working_folder.is_absolute() {
        return Err(TerminalError::RelativePath {
            field: "working_folder",
        });
    }
    if request.environment.iter().any(|(name, _)| name.is_empty()) {
        return Err(TerminalError::Empty {
            field: "environment name",
        });
    }
    if is_batch_script(&request.program) {
        return validate_batch_script(request);
    }
    Ok(())
}

/// A batch script runs through `cmd.exe`, so its path and its arguments are
/// both text that `cmd.exe` reads.
fn validate_batch_script(request: &SpawnTerminalRequest) -> Result<(), TerminalError> {
    if let Some(character) = first_unsafe_batch_character(&request.program) {
        return Err(TerminalError::UnsafeBatchProgram { character });
    }
    match first_unsafe_batch_argument(&request.arguments) {
        Some((index, character)) => Err(TerminalError::UnsafeBatchArgument { index, character }),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "spawn_terminal_validator.tests.rs"]
mod tests;
