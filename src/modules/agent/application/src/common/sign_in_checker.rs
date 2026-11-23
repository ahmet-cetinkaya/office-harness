// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use agent_domain::entities::Agent;
use agent_domain::value_objects::OfficeAdditions;

use crate::common::command_line::split_command;
use crate::common::continuation::adapter_of;
use crate::common::errors::ApplicationError;
use crate::common::interfaces::{CliAdapters, StatusCommandRunner};
use crate::common::models::{SignIn, StatusCommand, StatusRun};
use crate::common::sign_in_check::{
    NoAnswerReason, NotSignedInSource, SignInAnswer, answer, no_answer_note, not_signed_in_reason,
};
use crate::common::sign_in_scope::{SignInDecision, SignInScope};

/// Runs a CLI's sign-in status command for a start and decides what the
/// start does (0019): only a not-signed-in answer refuses it.
#[derive(Clone)]
pub struct SignInChecker {
    adapters: Arc<dyn CliAdapters>,
    status_commands: Arc<dyn StatusCommandRunner>,
    timeout: Duration,
}

/// A check that a start would run: what to run and how to read it.
struct PlannedCheck {
    sign_in: SignIn,
    command: StatusCommand,
    run: StatusRun,
}

impl SignInChecker {
    #[must_use]
    pub fn new(
        adapters: Arc<dyn CliAdapters>,
        status_commands: Arc<dyn StatusCommandRunner>,
        timeout: Duration,
    ) -> Self {
        Self {
            adapters,
            status_commands,
            timeout,
        }
    }

    /// The decision for a start that takes its turn now: the answer that a
    /// check before the turn left, when the agent is unchanged; a note when
    /// it changed; else a check run here (0019/FR-004, D3).
    ///
    /// # Errors
    /// `Invalid` when the agent's command cannot be split.
    pub fn decision_for_start(
        &self,
        agent: &Agent,
        folder: &Path,
        additions: &OfficeAdditions,
    ) -> Result<SignInDecision, ApplicationError> {
        let Some(planned) = self.planned(agent, folder, additions)? else {
            return Ok(SignInDecision::GoOn(None));
        };
        Ok(match SignInScope::take(agent.id) {
            Some((run, decision)) if run == planned.run => decision,
            Some(_) => SignInDecision::GoOn(Some(no_answer_note(&NoAnswerReason::AgentChanged))),
            None => self.decide(&planned),
        })
    }

    /// Runs the check before the start takes its turn, and keeps the answer
    /// for it on this thread (0019/FR-009, D13). `None` when no check runs.
    #[must_use]
    pub fn check_ahead(
        &self,
        agent: &Agent,
        folder: &Path,
        additions: &OfficeAdditions,
    ) -> Option<SignInScope> {
        let planned = self.planned(agent, folder, additions).ok()??;
        let decision = self.decide(&planned);
        Some(SignInScope::enter(agent.id, planned.run, decision))
    }

    /// The check of a start, or `None` when the CLI has no status command
    /// or the check could not judge the start: own arguments, flags, or a
    /// sign-in key variable in the additions (FR-010).
    fn planned(
        &self,
        agent: &Agent,
        folder: &Path,
        additions: &OfficeAdditions,
    ) -> Result<Option<PlannedCheck>, ApplicationError> {
        let sign_in = adapter_of(&*self.adapters, &agent.command)?.sign_in();
        let Some(command) = sign_in.status else {
            return Ok(None);
        };
        let (program, fixed_arguments) = split_command(&agent.command)?;
        let environment = environment_pairs(additions);
        let has_own_arguments = !fixed_arguments.is_empty() || !additions.flags().is_empty();
        if has_own_arguments || sets_key_variable(&environment, command.key_variables) {
            return Ok(None);
        }
        Ok(Some(PlannedCheck {
            sign_in,
            command,
            run: StatusRun {
                program,
                arguments: command
                    .arguments
                    .iter()
                    .map(|&argument| argument.to_owned())
                    .collect(),
                working_folder: folder.to_path_buf(),
                environment_additions: environment,
                key_variables: command.key_variables,
                timeout: self.timeout,
            },
        }))
    }

    fn decide(&self, planned: &PlannedCheck) -> SignInDecision {
        let result = self.status_commands.run(&planned.run);
        match answer(&result, &planned.command, self.timeout) {
            SignInAnswer::SignedIn | SignInAnswer::NotChecked => SignInDecision::GoOn(None),
            SignInAnswer::NotSignedIn { line } => SignInDecision::Refused(not_signed_in_reason(
                &planned.sign_in,
                NotSignedInSource::StatusCommand(&planned.command),
                &line,
            )),
            SignInAnswer::NoAnswer(reason) => SignInDecision::GoOn(Some(no_answer_note(&reason))),
        }
    }
}

/// The office additions' variables that have a value, as name and value.
#[must_use]
pub fn environment_pairs(additions: &OfficeAdditions) -> Vec<(String, String)> {
    additions
        .env()
        .iter()
        .filter_map(|entry| {
            entry
                .value()
                .map(|value| (entry.name().to_owned(), value.to_owned()))
        })
        .collect()
}

/// True when one of `key_variables` has a value that is not empty. Only
/// the names are compared; no value is read beyond its length.
#[must_use]
pub fn sets_key_variable(environment: &[(String, String)], key_variables: &[&str]) -> bool {
    environment
        .iter()
        .any(|(name, value)| !value.is_empty() && key_variables.contains(&name.as_str()))
}
