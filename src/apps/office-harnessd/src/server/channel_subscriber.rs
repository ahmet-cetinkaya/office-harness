// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;
use std::sync::Arc;
use terminal_application::common::interfaces::OutputSubscriber;
use terminal_application::common::models::{DeliveryStatus, LiveOutput};
use terminal_presentation::mappings::TerminalFrame;

use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{Notify, mpsc};

/// Hands the live output of one terminal to one connection as binary frames.
/// The channel is bounded and the delivery never waits: a connection that
/// falls behind is dropped from the terminal. Its view would freeze without
/// a sign, so the subscriber also signals the connection, which closes so the
/// client reconnects and attaches again to get a fresh snapshot.
pub struct ChannelSubscriber {
    agent_id: AgentId,
    frames: mpsc::Sender<Vec<u8>>,
    overflow: Arc<Notify>,
}

impl ChannelSubscriber {
    #[must_use]
    pub fn new(agent_id: AgentId, frames: mpsc::Sender<Vec<u8>>, overflow: Arc<Notify>) -> Self {
        Self {
            agent_id,
            frames,
            overflow,
        }
    }
}

impl OutputSubscriber for ChannelSubscriber {
    fn deliver(&self, output: LiveOutput) -> DeliveryStatus {
        let Ok(frame) = TerminalFrame::from_live(output).encode(self.agent_id) else {
            return DeliveryStatus::Closed;
        };
        match self.frames.try_send(frame) {
            Ok(()) => DeliveryStatus::Accepted,
            Err(TrySendError::Full(_)) => {
                // `notify_one` keeps the signal until the connection looks.
                self.overflow.notify_one();
                DeliveryStatus::Closed
            }
            Err(TrySendError::Closed(_)) => DeliveryStatus::Closed,
        }
    }
}

#[cfg(test)]
#[path = "channel_subscriber.tests.rs"]
mod tests;
