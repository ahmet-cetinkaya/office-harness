// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use agent_application::common::session_process_recorder::SessionProcessRecorder;
use tokio::sync::watch;
use tokio::task::spawn_blocking;
use tokio::time::{MissedTickBehavior, interval};

use crate::bootstrap::ServerLog;

/// How often the recorder scans the running sessions. It is below the 10
/// seconds in which a new helper must be stored (0017/FR-002, FR-011).
pub const SCAN_PERIOD: Duration = Duration::from_secs(5);

/// Scans the running sessions every `SCAN_PERIOD` until the server stops or
/// is dropped without serving. A scan runs on a blocking thread and the next
/// one waits for it, so scans never overlap.
pub fn spawn_process_scan(
    recorder: SessionProcessRecorder,
    mut stopping: watch::Receiver<bool>,
    log: ServerLog,
) {
    tokio::spawn(async move {
        let mut ticks = interval(SCAN_PERIOD);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = ticks.tick() => {}
                _ = stopping.changed() => return,
            }
            let scanning = recorder.clone();
            if let Err(error) = spawn_blocking(move || scanning.scan()).await {
                log.error(&format!("the process scan failed: {error}"));
            }
        }
    });
}
