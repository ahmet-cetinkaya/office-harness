// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::Clock;
use agent_infrastructure::persistence::{
    MaintenanceRepository, SessionProcessRepository, SharedConnection, WriteLogCheckpoint,
};

use super::ServerLog;
use crate::bridges::SystemClock;

/// The mark of the one-time rebuild that clears the display texts which
/// builds before 0023 freed (0023/FR-005).
const FREED_TEXT_CLEARED: &str = "freed_text_cleared";

/// Leaves no freed display text in the office database (0023/FR-004,
/// FR-005). It deletes the identities that the rules no longer keep, runs the
/// one-time rebuild when it has not finished yet, and truncates the write
/// log. It runs after the `lost` marking and before the sweep, and before the
/// server takes requests, because the rebuild needs no other connection
/// writing. A step that cannot finish writes its reason to the log and the
/// server starts anyway; the next start tries again (0023/FR-006).
pub fn clear_freed_text(connection: &SharedConnection, log: &ServerLog) {
    let maintenance = MaintenanceRepository::new(connection.clone());
    delete_unkept_identities(connection, log);
    let rebuild_pending = is_rebuild_pending(&maintenance, log);
    let rebuilt = !rebuild_pending || rebuild_file(&maintenance, log);
    // One checkpoint after the rebuild serves both: the rebuild leaves the old
    // pages in the log until it is truncated.
    let truncated = truncate_write_log(&maintenance, log);
    if rebuild_pending && rebuilt && truncated {
        mark_rebuild_done(&maintenance, log);
    }
}

fn delete_unkept_identities(connection: &SharedConnection, log: &ServerLog) {
    match SessionProcessRepository::new(connection.clone()).delete_unkept() {
        Ok(deleted) => log.info(&format!("process identities deleted at start: {deleted}")),
        Err(error) => log.error(&format!(
            "could not delete the process identities that are no longer kept: {error}"
        )),
    }
}

/// A mark that cannot be read counts as not pending, so that the start does
/// not run a rebuild that it cannot mark done; the next start reads it again.
fn is_rebuild_pending(maintenance: &MaintenanceRepository, log: &ServerLog) -> bool {
    match maintenance.is_done(FREED_TEXT_CLEARED) {
        Ok(done) => !done,
        Err(error) => {
            log.error(&format!("could not read the rebuild mark: {error}"));
            false
        }
    }
}

fn rebuild_file(maintenance: &MaintenanceRepository, log: &ServerLog) -> bool {
    match maintenance.rebuild_file() {
        Ok(()) => {
            log.info("office database rebuilt to clear freed text");
            true
        }
        Err(error) => {
            log.error(&format!("could not rebuild the office database: {error}"));
            false
        }
    }
}

fn truncate_write_log(maintenance: &MaintenanceRepository, log: &ServerLog) -> bool {
    match maintenance.truncate_write_log() {
        Ok(WriteLogCheckpoint::Truncated) => true,
        Ok(WriteLogCheckpoint::Busy) => {
            log.error(
                "could not truncate the write log: another connection holds a read transaction",
            );
            false
        }
        Err(error) => {
            log.error(&format!("could not truncate the write log: {error}"));
            false
        }
    }
}

fn mark_rebuild_done(maintenance: &MaintenanceRepository, log: &ServerLog) {
    if let Err(error) = maintenance.mark_done(FREED_TEXT_CLEARED, SystemClock.now()) {
        log.error(&format!("could not write the rebuild mark: {error}"));
    }
}
