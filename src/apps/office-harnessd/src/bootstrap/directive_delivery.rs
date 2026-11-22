// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use hierarchy_application::commands::send_directive::SendDirectiveHandler;
use tokio::task::spawn_blocking;

use super::ServerLog;
use crate::server::{EventHub, recorded_and_published};

/// How often waiting directives are looked at (0012 plan D6).
const DELIVERY_POLL: Duration = Duration::from_millis(250);

/// How long the thread sleeps while nothing waits before it checks again
/// that the server still runs.
const IDLE_CHECK: Duration = Duration::from_secs(1);

/// The reason a normal stop records for each waiting directive (0012/FR-005).
const SERVER_STOPPED: &str = "the office server stopped";

/// Starts the thread that types waiting directives once their target is
/// ready and withdraws the ones that may no longer be typed. The record
/// entries it writes reach the clients as `action.recorded`. The thread ends
/// soon after the handler is dropped.
pub fn spawn_directive_delivery(handler: &Arc<SendDirectiveHandler>, events: EventHub) {
    let handler = Arc::downgrade(handler);
    thread::spawn(move || {
        while let Some(handler) = handler.upgrade() {
            if handler.wait_for_waiting(IDLE_CHECK) {
                recorded_and_published(&events, || handler.deliver_waiting());
            }
            drop(handler);
            thread::sleep(DELIVERY_POLL);
        }
    });
}

/// Withdraws every waiting directive with the server-stop reason, so only a
/// crash leaves an accepted directive without an outcome (0012 A4).
pub async fn withdraw_waiting_directives(
    handler: &Arc<SendDirectiveHandler>,
    events: &EventHub,
    log: &ServerLog,
) {
    let handler = Arc::clone(handler);
    let events = events.clone();
    let withdrawn = spawn_blocking(move || {
        recorded_and_published(&events, || handler.withdraw_all(SERVER_STOPPED));
    })
    .await;
    if let Err(error) = withdrawn {
        log.error(&format!(
            "waiting directives could not be withdrawn: {error}"
        ));
    }
}
