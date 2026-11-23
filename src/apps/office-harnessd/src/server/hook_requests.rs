// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use access_application::common::interfaces::HookSecretStore;
use kernel::AgentId;
use serde::Deserialize;

/// The most reports of one session that the server takes in one second
/// (0024/FR-009).
pub const REPORTS_PER_SECOND: u32 = 50;

const RATE_WINDOW: Duration = Duration::from_secs(1);

/// A window that has not been touched for this long is dropped, so the
/// limiter holds state only for sessions that reported lately.
const IDLE_WINDOW_LIFETIME: Duration = Duration::from_secs(60);

/// The event kinds that a status hook reports (0024 Terms). A kind that the
/// server does not know is no event (0024/FR-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEventKind {
    TurnStarted,
    TurnStopped,
    TurnFailed,
    PermissionAsked,
    CompactionStarted,
    CompactionEnded,
}

impl HookEventKind {
    /// The kind for the name that the CLI gives its hook event.
    #[must_use]
    pub fn from_cli_event(name: &str) -> Option<Self> {
        match name {
            "UserPromptSubmit" => Some(Self::TurnStarted),
            "Stop" => Some(Self::TurnStopped),
            "StopFailure" => Some(Self::TurnFailed),
            "PermissionRequest" => Some(Self::PermissionAsked),
            "PreCompact" => Some(Self::CompactionStarted),
            "PostCompact" => Some(Self::CompactionEnded),
            _ => None,
        }
    }
}

/// What the server keeps of one report: the agent and the session it
/// belongs to, the event kind, and whether a helper sub-agent of the CLI
/// sent it (0024/FR-009, FR-023). Nothing else of the body survives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookReport {
    pub agent_id: AgentId,
    pub session: String,
    pub kind: HookEventKind,
    pub from_helper: bool,
}

/// Takes the reports that passed the checks of the endpoint.
pub trait HookReportSink: Send + Sync {
    fn accept(&self, report: HookReport);
}

/// The body fields that the server reads. Every other field of the body,
/// the prompt text first among them, is dropped by the parser.
#[derive(Deserialize)]
struct ReportBody {
    hook_event_name: Option<String>,
    agent_id: Option<serde_json::Value>,
}

/// The kind and the helper flag of a body, or `None` for a body that is not
/// a report of a known kind. No error text is made, so nothing of the body
/// can reach a log through it.
#[must_use]
pub fn read_report_body(body: &[u8]) -> Option<(HookEventKind, bool)> {
    let body: ReportBody = serde_json::from_slice(body).ok()?;
    let kind = HookEventKind::from_cli_event(body.hook_event_name.as_deref()?)?;
    let from_helper = body.agent_id.is_some_and(|value| !value.is_null());
    Some((kind, from_helper))
}

struct Window {
    started: Instant,
    count: u32,
}

/// Counts the reports of each session in one-second windows.
#[derive(Default)]
struct RateLimiter {
    windows: HashMap<String, Window>,
}

impl RateLimiter {
    fn admit(&mut self, session: &str, now: Instant) -> bool {
        self.windows.retain(|_, window| {
            now.saturating_duration_since(window.started) < IDLE_WINDOW_LIFETIME
        });
        let window = self.windows.entry(session.to_owned()).or_insert(Window {
            started: now,
            count: 0,
        });
        if now.saturating_duration_since(window.started) >= RATE_WINDOW {
            *window = Window {
                started: now,
                count: 0,
            };
        }
        if window.count >= REPORTS_PER_SECOND {
            return false;
        }
        window.count += 1;
        true
    }
}

/// The report endpoint's checks and the hand-over to the sink (0024). A
/// refusal of any kind is one answer to the caller (0024/FR-008).
#[derive(Clone)]
pub struct HookGateway {
    secrets: Arc<dyn HookSecretStore>,
    sink: Arc<dyn HookReportSink>,
    limiter: Arc<Mutex<RateLimiter>>,
}

impl HookGateway {
    #[must_use]
    pub fn new(secrets: Arc<dyn HookSecretStore>, sink: Arc<dyn HookReportSink>) -> Self {
        Self {
            secrets,
            sink,
            limiter: Arc::default(),
        }
    }

    /// The agent of `session` when `secret` is that session's secret.
    #[must_use]
    pub fn authorize(&self, session: &str, secret: &str) -> Option<AgentId> {
        self.secrets.authorize(session, secret)
    }

    /// Whether one more report of the authorized session fits in the rate.
    /// Only authorized sessions reach it, so a flood of bad reports adds no
    /// state.
    #[must_use]
    pub fn admit(&self, session: &str) -> bool {
        self.limiter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .admit(session, Instant::now())
    }

    /// Hands the body's report to the sink. A body with an unknown kind, or
    /// one that is not JSON, is no event.
    pub fn deliver(&self, agent_id: AgentId, session: &str, body: &[u8]) {
        let Some((kind, from_helper)) = read_report_body(body) else {
            return;
        };
        self.sink.accept(HookReport {
            agent_id,
            session: session.to_owned(),
            kind,
            from_helper,
        });
    }
}

/// A sink that drops every report, until the status bridge takes them.
pub struct DiscardHookReports;

impl HookReportSink for DiscardHookReports {
    fn accept(&self, _report: HookReport) {}
}

#[cfg(test)]
#[path = "hook_requests.tests.rs"]
mod tests;
