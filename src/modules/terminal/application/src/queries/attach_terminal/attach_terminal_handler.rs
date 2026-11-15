// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use kernel::AgentId;
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use crate::common::errors::TerminalRuntimeError;
use crate::common::interfaces::{OutputSubscriber, TerminalRuntime};
use crate::common::models::{Attachment, SubscriptionId};

pub struct AttachTerminalHandler {
    runtime: Arc<dyn TerminalRuntime>,
}

impl AttachTerminalHandler {
    #[must_use]
    pub fn new(runtime: Arc<dyn TerminalRuntime>) -> Self {
        Self { runtime }
    }

    /// `saved_output` names the output of the agent's last session; it is used
    /// only when the agent is not running.
    ///
    /// # Errors
    /// `NoOutput` when there is nothing to show.
    pub fn attach(
        &self,
        agent_id: AgentId,
        saved_output: Option<&OutputKey>,
        size: TerminalSize,
        subscriber: Arc<dyn OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError> {
        self.runtime
            .attach(agent_id, saved_output, size, subscriber)
    }

    pub fn detach(&self, agent_id: AgentId, subscription: SubscriptionId) {
        self.runtime.detach(agent_id, subscription);
    }
}
