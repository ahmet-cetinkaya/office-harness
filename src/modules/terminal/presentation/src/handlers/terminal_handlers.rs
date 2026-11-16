// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use kernel::AgentId;
use terminal_application::commands::resize_terminal::ResizeTerminalHandler;
use terminal_application::commands::send_terminal_input::SendTerminalInputHandler;
use terminal_application::common::interfaces::{OutputSubscriber, TerminalRuntime};
use terminal_application::common::models::SubscriptionId;
use terminal_application::queries::attach_terminal::AttachTerminalHandler;
use terminal_domain::errors::TerminalError;
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use crate::contracts::{AttachRequest, DetachRequest, InputRequest, ResizeRequest};
use crate::handlers::HandlerError;
use crate::mappings::TerminalFrame;

/// What `terminal.attach` produces: the snapshot frame to send first, and the
/// subscription to detach later. A saved snapshot has no subscription.
#[derive(Debug)]
pub struct AttachedTerminal {
    pub snapshot_frame: Vec<u8>,
    pub subscription: Option<SubscriptionId>,
}

/// The handlers of the `terminal.*` messages. Each one parses its body, names
/// the field at fault, and calls the use case.
pub struct TerminalHandlers {
    attach: AttachTerminalHandler,
    input: SendTerminalInputHandler,
    resize: ResizeTerminalHandler,
}

impl TerminalHandlers {
    #[must_use]
    pub fn new(runtime: &Arc<dyn TerminalRuntime>) -> Self {
        Self {
            attach: AttachTerminalHandler::new(Arc::clone(runtime)),
            input: SendTerminalInputHandler::new(Arc::clone(runtime)),
            resize: ResizeTerminalHandler::new(Arc::clone(runtime)),
        }
    }

    /// `saved_output` is the output file of the agent's last session, which
    /// the host reads from the agent module. It is used only when the agent
    /// is not running.
    ///
    /// # Errors
    /// `invalid` for a bad field, `not_found` when there is nothing to show.
    pub fn attach(
        &self,
        request: &AttachRequest,
        saved_output: Option<&OutputKey>,
        subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<AttachedTerminal, HandlerError> {
        let agent_id = parse_agent_id(&request.agent_id)?;
        let size = TerminalSize::new(request.cols, request.rows)?;
        let attachment = self
            .attach
            .attach(agent_id, saved_output, size, subscriber)?;
        Ok(AttachedTerminal {
            snapshot_frame: TerminalFrame::Snapshot(attachment.snapshot).encode(agent_id)?,
            subscription: attachment.subscription,
        })
    }

    /// # Errors
    /// `invalid` when `agent_id` is not an id.
    pub fn detach(
        &self,
        request: &DetachRequest,
        subscription: SubscriptionId,
    ) -> Result<(), HandlerError> {
        self.attach
            .detach(parse_agent_id(&request.agent_id)?, subscription);
        Ok(())
    }

    /// # Errors
    /// `invalid` for a bad field, `not_found` when the agent is not running.
    pub fn input(&self, request: &InputRequest) -> Result<(), HandlerError> {
        let agent_id = parse_agent_id(&request.agent_id)?;
        let bytes = STANDARD
            .decode(&request.data)
            .map_err(|_| TerminalError::InvalidEncoding { field: "data" })?;
        Ok(self.input.handle(agent_id, &bytes)?)
    }

    /// # Errors
    /// `invalid` for a bad field, `not_found` when the agent is not running.
    pub fn resize(&self, request: &ResizeRequest) -> Result<(), HandlerError> {
        let agent_id = parse_agent_id(&request.agent_id)?;
        let size = TerminalSize::new(request.cols, request.rows)?;
        self.resize.handle(agent_id, size)?;
        Ok(())
    }
}

fn parse_agent_id(text: &str) -> Result<AgentId, TerminalError> {
    text.parse().map_err(|_| TerminalError::InvalidId {
        field: "agent_id",
        text: text.to_owned(),
    })
}

#[cfg(test)]
#[path = "terminal_handlers.tests.rs"]
mod tests;
