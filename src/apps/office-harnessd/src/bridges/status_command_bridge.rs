// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_application::common::interfaces::StatusCommandRunner;
use agent_application::common::models::{StatusRun, StatusRunResult};
use agent_domain::value_objects::RESERVED_ENV_PREFIX;
use agent_infrastructure::services::{Environment, run_status_command};

use super::terminal_launcher::{PrepareRequest, TerminalLauncher};

/// The agent module's `StatusCommandRunner`: the status command gets the
/// program and the login environment that the agent's session would get,
/// with its office additions (0019 D7). The office's own variables, the
/// agent-call credential among them, never reach it: the credential is only
/// issued later, for the session, and every reserved name is dropped here.
pub struct StatusCommandBridge {
    launcher: TerminalLauncher,
}

impl StatusCommandBridge {
    #[must_use]
    pub fn new(launcher: TerminalLauncher) -> Self {
        Self { launcher }
    }
}

impl StatusCommandRunner for StatusCommandBridge {
    fn run(&self, run: &StatusRun) -> StatusRunResult {
        let prepared = match self.launcher.prepare(&PrepareRequest {
            program: run.program.clone(),
            working_folder: run.working_folder.clone(),
            additions: run.environment_additions.clone(),
        }) {
            Ok(prepared) => prepared,
            Err(error) => return StatusRunResult::CouldNotRun(error.to_string()),
        };
        let environment: Environment = prepared
            .environment
            .into_iter()
            .filter(|(name, _)| !name.starts_with(RESERVED_ENV_PREFIX))
            .collect();
        // A key in the login environment can sign the CLI in where its status
        // command may not see it (0019/FR-010): no run, the start goes on.
        let sets_key = run.key_variables.iter().any(|name| {
            environment
                .get(*name)
                .is_some_and(|value| !value.is_empty())
        });
        if sets_key {
            return StatusRunResult::Skipped;
        }
        run_status_command(
            &prepared.program,
            &run.arguments,
            &run.working_folder,
            &environment,
            run.timeout,
        )
    }
}
