// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use crate::common::models::{SessionCommand, SessionPlan, TrackedSession, TrackingRequest};

/// Follows a started CLI until its own session id is known.
pub trait SessionTracking: Send {
    /// Starts looking in the background and calls `report` once, when the
    /// session is matched or when matching gave up.
    fn follow(self: Box<Self>, report: Box<dyn FnOnce(TrackedSession) + Send>);
}

/// What one CLI needs to start a new session or to continue one (research
/// R3). An adapter only builds command lines and reads the CLI's session
/// files; it never writes a CLI file (FR-021).
pub trait CliAdapter: Send + Sync {
    fn supports_continue(&self) -> bool;

    /// The command line of a new session (FR-024).
    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand;

    /// The command line that continues the CLI's own session, or `None` when
    /// the CLI cannot continue one (FR-025).
    fn continue_session(
        &self,
        plan: &SessionPlan<'_>,
        cli_session_id: &str,
    ) -> Option<SessionCommand>;

    /// For a CLI that does not accept a chosen id: starts to watch for the
    /// session that the next start creates. Called before the start.
    fn track_new_session(
        &self,
        _request: &TrackingRequest<'_>,
    ) -> Option<Box<dyn SessionTracking>> {
        None
    }
}

/// Chooses the adapter for a program name.
pub trait CliAdapters: Send + Sync {
    /// `program` is the first word of the agent's command: a bare name, or a
    /// path. An unknown program gets the generic adapter.
    fn for_program(&self, program: &str) -> Arc<dyn CliAdapter>;
}
