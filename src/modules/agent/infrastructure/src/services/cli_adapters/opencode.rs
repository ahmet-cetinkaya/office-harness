// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{SystemTime, UNIX_EPOCH};

use agent_application::common::interfaces::CliAdapter;
use agent_application::common::models::{SessionCommand, SessionPlan};

use super::command_builder::{continue_command, new_session_command};

/// `opencode` refuses a session id that does not start with this prefix
/// ("Expected a string starting with \"ses\"", version 2.2.5).
const SESSION_ID_PREFIX: &str = "ses_";

/// The CLI's own ids hold 12 hex digits of an inverted time counter, then 14
/// letters and digits. The counter is the millisecond clock times this many
/// slots, plus a slot number, cut to 48 bits.
const COUNTER_SLOTS_PER_MILLISECOND: u64 = 0x1000;
const COUNTER_MASK: u64 = (1 << 48) - 1;
const SLOT_DIGITS: usize = 3;
const TAIL_DIGITS: usize = 14;

/// `opencode`: `--session <id>` creates the session when it does not exist
/// and continues it when it does. A new session gets an id in the CLI's own
/// shape, because the free tier of 2.2.5 answers "Free usage limit reached"
/// to an id whose first 12 digits are not a counter near the present time.
/// The id is recorded when the session starts and a continue passes the
/// recorded id unchanged, so an id of an older shape (`ses_` and 32 hex
/// digits) still resumes.
pub struct OpencodeAdapter;

impl CliAdapter for OpencodeAdapter {
    fn supports_continue(&self) -> bool {
        true
    }

    fn new_session(&self, plan: &SessionPlan<'_>) -> SessionCommand {
        let cli_session_id = opencode_session_id(plan.proposed_cli_session_id, now_millis());
        new_session_command(
            plan,
            &["--session", &cli_session_id],
            Some(cli_session_id.clone()),
        )
    }

    fn continue_session(
        &self,
        plan: &SessionPlan<'_>,
        cli_session_id: &str,
    ) -> Option<SessionCommand> {
        Some(continue_command(plan, &["--session", cli_session_id]))
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
        .unwrap_or(0)
}

/// The proposed id supplies the slot number and the 14-digit tail, so the
/// same session id and the same clock reading give the same CLI id.
fn opencode_session_id(proposed_cli_session_id: &str, now_millis: u64) -> String {
    let mut digits = proposed_cli_session_id
        .chars()
        .filter(char::is_ascii_hexdigit)
        .chain(std::iter::repeat('0'));
    let slot: String = digits.by_ref().take(SLOT_DIGITS).collect();
    let tail: String = digits.take(TAIL_DIGITS).collect();
    let slot = u64::from_str_radix(&slot, 16).unwrap_or(0);
    let counter = now_millis
        .wrapping_mul(COUNTER_SLOTS_PER_MILLISECOND)
        .wrapping_add(slot);
    let inverted = !counter & COUNTER_MASK;
    format!("{SESSION_ID_PREFIX}{inverted:012x}{tail}")
}

#[cfg(test)]
#[path = "opencode.tests.rs"]
mod tests;
