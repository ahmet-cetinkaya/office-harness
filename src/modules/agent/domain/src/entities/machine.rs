// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::AgentError;
use crate::value_objects::MachineId;

/// The computer the office runs on. The MVP has exactly one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub id: MachineId,
    pub name: String,
}

impl Machine {
    /// # Errors
    /// `Empty` when the host name is blank.
    pub fn new(id: MachineId, name: &str) -> Result<Self, AgentError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AgentError::Empty {
                field: "machine name",
            });
        }
        Ok(Self {
            id,
            name: trimmed.to_owned(),
        })
    }
}
