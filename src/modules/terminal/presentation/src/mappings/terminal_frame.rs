// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use terminal_application::common::models::LiveOutput;
use terminal_domain::errors::TerminalError;
use uuid::Uuid;

const SNAPSHOT_KIND: u8 = 0x01;
const OUTPUT_KIND: u8 = 0x02;
const DROPPED_KIND: u8 = 0x03;

/// A binary frame of terminal data: one byte of kind, 16 bytes of agent UUID,
/// then the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalFrame {
    /// Sent once after `terminal.attach`.
    Snapshot(Vec<u8>),
    /// Live output.
    Output(Vec<u8>),
    /// Older output was dropped. It has no payload.
    OlderOutputDropped,
}

impl TerminalFrame {
    #[must_use]
    pub fn from_live(output: LiveOutput) -> Self {
        match output {
            LiveOutput::Bytes(bytes) => Self::Output(bytes),
            LiveOutput::OlderOutputDropped => Self::OlderOutputDropped,
        }
    }

    /// # Errors
    /// `InvalidId` when the agent id is not a UUID.
    pub fn encode(&self, agent_id: AgentId) -> Result<Vec<u8>, TerminalError> {
        let text = agent_id.to_string();
        let uuid = Uuid::parse_str(&text).map_err(|_| TerminalError::InvalidId {
            field: "agent_id",
            text,
        })?;
        let (kind, payload): (u8, &[u8]) = match self {
            Self::Snapshot(bytes) => (SNAPSHOT_KIND, bytes),
            Self::Output(bytes) => (OUTPUT_KIND, bytes),
            Self::OlderOutputDropped => (DROPPED_KIND, &[]),
        };
        let mut frame = Vec::with_capacity(1 + 16 + payload.len());
        frame.push(kind);
        frame.extend_from_slice(uuid.as_bytes());
        frame.extend_from_slice(payload);
        Ok(frame)
    }
}

#[cfg(test)]
#[path = "terminal_frame.tests.rs"]
mod tests;
