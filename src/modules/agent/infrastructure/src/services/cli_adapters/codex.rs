// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use agent_application::common::interfaces::{CliAdapter, SessionTracking};
use agent_application::common::models::{SessionCommand, SessionPlan, TrackingRequest};

use super::codex_sessions::{AttributionTiming, CodexTracking, PendingStarts, sessions_folder};
use super::command_builder::{PromptStyle, continue_command, new_session_command};

/// `codex`: it does not take an id at the start, so the id is found by
/// reading the CLI's session files before and after the start (read only,
/// FR-021). A session continues with `codex resume <id>`.
pub struct CodexAdapter {
    timing: AttributionTiming,
    pending: Arc<PendingStarts>,
}

impl CodexAdapter {
    #[must_use]
    pub fn new(timing: AttributionTiming) -> Self {
        Self {
            timing,
            pending: Arc::new(PendingStarts::default()),
        }
    }
}

impl CliAdapter for CodexAdapter {
    fn supports_continue(&self) -> bool {
        true
    }

    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand {
        new_session_command(plan, &[], PromptStyle::Positional, None)
    }

    fn continue_session(
        &self,
        plan: &SessionPlan<'_>,
        cli_session_id: &str,
    ) -> Option<SessionCommand> {
        Some(continue_command(plan, &["resume", cli_session_id]))
    }

    fn track_new_session(&self, request: &TrackingRequest<'_>) -> Option<Box<dyn SessionTracking>> {
        let folder = sessions_folder(request.environment)?;
        Some(Box::new(CodexTracking::begin(
            folder,
            request.working_folder,
            self.timing,
            &self.pending,
        )))
    }
}
