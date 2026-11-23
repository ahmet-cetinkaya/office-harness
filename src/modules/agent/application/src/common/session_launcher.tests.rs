// SPDX-License-Identifier: AGPL-3.0-or-later

//! The sign-in check of every start (0019): the launcher runs the status
//! command before it asks the terminal host for anything.

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::ExitResult;
use tempfile::TempDir;

use agent_domain::entities::AdditionsRecord;
use agent_domain::value_objects::{EnvEntry, OfficeAdditions};
use kernel::{AgentId, Timestamp};

use crate::common::errors::ApplicationError;
use crate::common::interfaces::{AdditionsStore, AgentStore, SessionStore};
use crate::common::models::{AgentAction, StatusRunResult};
use crate::common::sign_in_check::SIGN_IN_TIMEOUT;
use crate::test_support::Office;

const NOT_SIGNED_IN_OUTPUT: &str =
    "{\n  \"token\": \"sk-not-shown\"\n}\nNot logged in. Sign in first.";
const NOT_SIGNED_IN_REASON: &str = "signin is not signed in: `signin auth status` said so. Sign in \
     to signin in your own terminal, then start the agent again. It showed: Not logged in. Sign \
     in first.";

fn not_signed_in(office: &Office) {
    office.status_commands.answer_with(StatusRunResult::Ended {
        exit_code: Some(1),
        output: NOT_SIGNED_IN_OUTPUT.to_owned(),
    });
}

fn agent_with(
    office: &Office,
    command: &str,
    status: AgentStatus,
) -> agent_domain::entities::Agent {
    let folder = TempDir::new().expect("folder");
    let name = format!("Agent {}", office.agents.list().expect("list").len());
    office.add_agent(&name, command, folder.keep().as_path(), status)
}

/// A command, its flags, and its variables, for a start that skips the check.
type SkipCase = (
    &'static str,
    Vec<String>,
    &'static [(&'static str, &'static str)],
);

fn save_additions(
    office: &Office,
    agent_id: AgentId,
    flags: Vec<String>,
    variables: &[(&str, &str)],
) {
    let env = variables
        .iter()
        .map(|(name, value)| EnvEntry::parse(name, Some(value)).expect("entry"))
        .collect();
    let additions = OfficeAdditions::new(flags, env, None).expect("additions");
    office
        .additions
        .save(
            agent_id,
            &AdditionsRecord {
                additions,
                updated_at: Timestamp::from_unix_millis(0),
            },
        )
        .expect("save");
}

fn assert_failed_without_a_terminal(
    office: &Office,
    agent_id: kernel::AgentId,
    failure: Option<&str>,
) {
    assert_eq!(failure, Some(NOT_SIGNED_IN_REASON));
    assert_eq!(office.agents.status_of(agent_id), Some(AgentStatus::Failed));
    assert_eq!(office.terminals.started().len(), 0);
    // The credential is issued in the host's prepare, which never ran.
    assert_eq!(office.terminals.prepared().len(), 0);
    let last = office
        .sessions
        .latest_for_agent(agent_id)
        .expect("read")
        .expect("session");
    assert_eq!(
        last.exit_result,
        Some(ExitResult::SpawnError(NOT_SIGNED_IN_REASON.to_owned()))
    );
}

#[test]
fn a_start_of_a_cli_that_is_not_signed_in_fails_with_the_reason_and_starts_nothing() {
    let office = Office::new();
    not_signed_in(&office);
    let agent = agent_with(&office, "signin", AgentStatus::Stopped);

    let outcome = office.start_agent().handle(agent.id).expect("start");

    assert_failed_without_a_terminal(&office, agent.id, outcome.failure.as_deref());
    let entries = office.recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, AgentAction::Start);
    assert_eq!(entries[0].failure.as_deref(), Some(NOT_SIGNED_IN_REASON));
    assert_eq!(entries[0].detail, None);
}

#[test]
fn the_status_command_runs_with_the_program_folder_and_additions_only() {
    let office = Office::new();
    let agent = agent_with(&office, "signin", AgentStatus::Stopped);

    office.start_agent().handle(agent.id).expect("start");

    let runs = office.status_commands.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].program, "signin");
    assert_eq!(runs[0].arguments, ["auth", "status"]);
    assert_eq!(Some(runs[0].working_folder.clone()), agent.working_folder);
    assert_eq!(runs[0].timeout, SIGN_IN_TIMEOUT);
    assert_eq!(office.terminals.started().len(), 1);
}

#[test]
fn a_new_task_on_a_running_agent_is_refused_and_the_running_session_goes_on() {
    let office = Office::new();
    let agent = agent_with(&office, "signin", AgentStatus::Stopped);
    let running = office
        .start_agent()
        .handle(agent.id)
        .expect("first start")
        .session;
    not_signed_in(&office);

    let refusal = office
        .start_new_task()
        .handle(agent.id)
        .expect_err("refused");

    assert_eq!(
        refusal,
        ApplicationError::Refused(NOT_SIGNED_IN_REASON.to_owned())
    );
    assert_eq!(office.terminals.stopped().len(), 0);
    assert_eq!(office.terminals.started().len(), 1);
    assert_eq!(
        office.agents.status_of(agent.id),
        Some(AgentStatus::Starting)
    );
    let latest = office
        .sessions
        .latest_for_agent(agent.id)
        .expect("read")
        .expect("session");
    assert_eq!(latest.id, running.id);
    assert!(latest.is_running());
    let entries = office.recorder.entries();
    let last = entries.last().expect("entry");
    assert_eq!(last.action, AgentAction::NewTask);
    assert_eq!(last.failure.as_deref(), Some(NOT_SIGNED_IN_REASON));
}

#[test]
fn a_continue_on_a_running_agent_is_refused_and_the_running_session_goes_on() {
    let office = Office::new();
    let agent = agent_with(&office, "signin", AgentStatus::Stopped);
    let first = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office
        .end_session()
        .handle(agent.id, &ExitResult::Code(0))
        .expect("end");
    office
        .agents
        .update_status(agent.id, AgentStatus::Exited)
        .expect("status");
    office.start_agent().handle(agent.id).expect("second start");
    not_signed_in(&office);

    let refusal = office
        .continue_session()
        .handle(first.id)
        .expect_err("refused");

    assert_eq!(
        refusal,
        ApplicationError::Refused(NOT_SIGNED_IN_REASON.to_owned())
    );
    assert_eq!(office.terminals.stopped().len(), 0);
    assert_eq!(office.terminals.started().len(), 2);
}

#[test]
fn an_answer_checked_ahead_is_used_without_a_second_run() {
    let office = Office::new();
    let agent = agent_with(&office, "signin", AgentStatus::Stopped);
    not_signed_in(&office);
    let launcher = office.launcher();

    let scope = launcher.check_sign_in_ahead(agent.id).expect("checked");
    office.status_commands.answer_with(StatusRunResult::Ended {
        exit_code: Some(0),
        output: String::new(),
    });
    let outcome = office.start_agent().handle(agent.id).expect("start");
    drop(scope);

    assert_eq!(outcome.failure.as_deref(), Some(NOT_SIGNED_IN_REASON));
    assert_eq!(office.status_commands.runs().len(), 1);
}

#[test]
fn an_agent_that_changed_after_the_check_ahead_starts_with_a_note() {
    let office = Office::new();
    let agent = agent_with(&office, "signin", AgentStatus::Stopped);
    not_signed_in(&office);
    let scope = office
        .launcher()
        .check_sign_in_ahead(agent.id)
        .expect("checked");
    save_additions(&office, agent.id, Vec::new(), &[("OTHER", "value")]);

    let outcome = office.start_agent().handle(agent.id).expect("start");
    drop(scope);

    assert_eq!(outcome.failure, None);
    assert_eq!(
        outcome.sign_in_note.as_deref(),
        Some("the sign-in check gave no answer: the agent changed during the check")
    );
    assert_eq!(office.status_commands.runs().len(), 1);
}

#[test]
fn own_arguments_flags_or_a_key_variable_skip_the_check() {
    let cases: [SkipCase; 3] = [
        ("signin --settings mine.json", Vec::new(), &[]),
        ("signin", vec!["--profile".to_owned()], &[]),
        ("signin", Vec::new(), &[("SIGNIN_API_KEY", "set")]),
    ];
    for (command, flags, variables) in cases {
        let office = Office::new();
        not_signed_in(&office);
        let agent = agent_with(&office, command, AgentStatus::Stopped);
        save_additions(&office, agent.id, flags, variables);

        let outcome = office.start_agent().handle(agent.id).expect("start");

        assert_eq!(office.status_commands.runs().len(), 0, "{command}");
        assert_eq!(outcome.failure, None);
        assert_eq!(outcome.sign_in_note, None);
        assert_eq!(office.terminals.started().len(), 1);
    }
}

#[test]
fn an_empty_key_variable_does_not_skip_the_check() {
    let office = Office::new();
    let agent = agent_with(&office, "signin", AgentStatus::Stopped);
    save_additions(&office, agent.id, Vec::new(), &[("SIGNIN_API_KEY", "")]);

    office.start_agent().handle(agent.id).expect("start");

    assert_eq!(office.status_commands.runs().len(), 1);
}

#[test]
fn a_continue_of_a_cli_that_is_not_signed_in_fails_with_the_reason() {
    let office = Office::new();
    let agent = agent_with(&office, "signin", AgentStatus::Stopped);
    let first = office
        .start_agent()
        .handle(agent.id)
        .expect("start")
        .session;
    office
        .end_session()
        .handle(agent.id, &ExitResult::Code(0))
        .expect("end");
    office
        .agents
        .update_status(agent.id, AgentStatus::Exited)
        .expect("status");
    not_signed_in(&office);

    let outcome = office
        .continue_session()
        .handle(first.id)
        .expect("continue");

    assert_eq!(outcome.failure.as_deref(), Some(NOT_SIGNED_IN_REASON));
    assert_eq!(office.agents.status_of(agent.id), Some(AgentStatus::Failed));
    assert_eq!(office.terminals.started().len(), 1);
}

#[test]
fn a_check_without_an_answer_lets_the_start_go_on_with_a_note() {
    for (result, note) in [
        (
            StatusRunResult::TimedOut,
            "the sign-in check gave no answer: it timed out after 5 s",
        ),
        (
            StatusRunResult::CouldNotRun("not found".to_owned()),
            "the sign-in check gave no answer: it could not run: not found",
        ),
        (
            StatusRunResult::Ended {
                exit_code: Some(2),
                output: "usage: signin [OPTIONS] secret-looking-text".to_owned(),
            },
            "the sign-in check gave no answer: its answer was unclear",
        ),
    ] {
        let office = Office::new();
        office.status_commands.answer_with(result);
        let agent = agent_with(&office, "signin", AgentStatus::Stopped);

        let outcome = office.start_agent().handle(agent.id).expect("start");

        assert_eq!(outcome.failure, None);
        assert_eq!(outcome.sign_in_note.as_deref(), Some(note));
        assert_eq!(office.terminals.started().len(), 1);
        let entries = office.recorder.entries();
        assert_eq!(entries[0].detail.as_deref(), Some(note));
        assert_eq!(entries[0].failure, None);
    }
}

#[test]
fn a_cli_without_a_status_command_runs_no_check() {
    let office = Office::new();
    not_signed_in(&office);
    let agent = agent_with(&office, "claude", AgentStatus::Stopped);

    let outcome = office.start_agent().handle(agent.id).expect("start");

    assert_eq!(office.status_commands.runs().len(), 0);
    assert_eq!(outcome.failure, None);
    assert_eq!(outcome.sign_in_note, None);
    assert_eq!(office.terminals.started().len(), 1);
}

#[test]
fn the_terminal_gets_the_sign_in_screen_words_of_the_cli() {
    let office = Office::new();
    let watched = agent_with(&office, "signin", AgentStatus::Stopped);
    let unwatched = agent_with(&office, "claude", AgentStatus::Stopped);

    office.start_agent().handle(watched.id).expect("start");
    office.start_agent().handle(unwatched.id).expect("start");

    let started = office.terminals.started();
    assert_eq!(started[0].sign_in.screen_words, ["please sign in"]);
    assert_eq!(started[1].sign_in.screen_words.len(), 0);
}

mod status_hooks {
    use super::super::*;
    use crate::common::models::SessionCommand;

    struct Hooked(bool);

    impl CliAdapter for Hooked {
        fn supports_continue(&self) -> bool {
            false
        }

        fn new_session(&self, _plan: &SessionPlan<'_>) -> SessionCommand {
            SessionCommand {
                arguments: Vec::new(),
                cli_session_id: None,
                typed_input: None,
            }
        }

        fn continue_session(
            &self,
            _plan: &SessionPlan<'_>,
            _cli_session_id: &str,
        ) -> Option<SessionCommand> {
            None
        }

        fn status_hooks(&self) -> bool {
            self.0
        }
    }

    fn words(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    #[test]
    fn a_cli_without_hooks_is_unsupported() {
        assert_eq!(
            status_hooks_request(&Hooked(false), &[], &[]),
            StatusHooksRequest::Unsupported
        );
    }

    #[test]
    fn a_cli_with_hooks_and_no_settings_option_wants_the_file() {
        let flags = words(&["--model", "opus"]);
        assert_eq!(
            status_hooks_request(&Hooked(true), &words(&["--verbose"]), &flags),
            StatusHooksRequest::Wanted
        );
    }

    #[test]
    fn a_settings_option_in_the_command_or_the_flags_refuses_the_file() {
        let own = StatusHooksRequest::Refused(OWN_SETTINGS_OPTION_NOTE.to_owned());
        for (fixed, flags) in [
            (words(&["--settings", "x.json"]), words(&[])),
            (words(&["--settings=x.json"]), words(&[])),
            (words(&[]), words(&["--settings", "x.json"])),
            (words(&[]), words(&["--settings={}"])),
        ] {
            assert_eq!(status_hooks_request(&Hooked(true), &fixed, &flags), own);
        }
        assert_eq!(
            status_hooks_request(&Hooked(true), &words(&["--settings-extra"]), &[]),
            StatusHooksRequest::Wanted
        );
    }
}
