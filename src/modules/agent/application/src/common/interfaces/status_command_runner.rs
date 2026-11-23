// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::common::models::{StatusRun, StatusRunResult};

/// Runs a CLI's read-only sign-in status command as the agent's session
/// would run it, but without input, without a terminal, and without the
/// office's agent-call credential. It reads and writes no file and keeps
/// the output in memory only (0019/FR-006).
pub trait StatusCommandRunner: Send + Sync {
    fn run(&self, run: &StatusRun) -> StatusRunResult;
}
