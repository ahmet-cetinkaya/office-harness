// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use agent_application::common::errors::PortError;
use agent_application::common::interfaces::{ActionRecorder, AgentStore};
use agent_application::common::models::{AgentAction, AgentActionEntry};
use agent_domain::entities::Agent;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId};
use hierarchy_application::common::ports::{DirectiveJournal, DirectiveTarget, DirectiveWriter};
use kernel::{AgentId, Timestamp};
use terminal_application::common::errors::TerminalRuntimeError;
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_application::common::models::{
    Attachment, RunId, SpawnTerminalRequest, SubscriptionId,
};
use terminal_domain::value_objects::{OutputKey, TerminalSize};

use super::*;
use crate::bridges::SystemClock;

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn agent_with_status(agent_id: AgentId, name: &str, status: AgentStatus) -> Agent {
    let mut agent = Agent::new(
        agent_id,
        DisplayName::parse("name", name).expect("name"),
        AgentCommand::parse("fake-cli").expect("command"),
        Some(std::env::temp_dir()),
        None,
        MachineId::generate(),
        Timestamp::from_unix_millis(1_700_000_000_000),
    )
    .expect("agent");
    agent.status = status;
    agent
}

/// The one agent these tests look up, by its own id; another id is unknown.
struct OneAgent(Agent);

impl AgentStore for OneAgent {
    fn insert(&self, _agent: &Agent) -> Result<(), PortError> {
        Ok(())
    }

    fn find(&self, id: AgentId) -> Result<Option<Agent>, PortError> {
        Ok((id == self.0.id).then(|| self.0.clone()))
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Ok(Vec::new())
    }

    fn update_status(&self, _id: AgentId, _status: AgentStatus) -> Result<(), PortError> {
        Ok(())
    }

    fn delete(&self, _id: AgentId) -> Result<(), PortError> {
        Ok(())
    }
}

/// A runtime that keeps every `write_input` call and is running for one id.
struct RecordingRuntime {
    written: Mutex<Vec<(AgentId, Vec<u8>)>>,
    running_for: AgentId,
}

impl TerminalRuntime for RecordingRuntime {
    fn spawn(&self, _request: &SpawnTerminalRequest) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn attach(
        &self,
        _agent_id: AgentId,
        _saved_output: Option<&OutputKey>,
        _size: TerminalSize,
        _subscriber: Arc<dyn terminal_application::common::interfaces::OutputSubscriber>,
    ) -> Result<Attachment, TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(self.running_for))
    }

    fn detach(&self, _agent_id: AgentId, _subscription: SubscriptionId) {}

    fn write_input(&self, agent_id: AgentId, bytes: &[u8]) -> Result<(), TerminalRuntimeError> {
        locked(&self.written).push((agent_id, bytes.to_vec()));
        Ok(())
    }

    fn paste(&self, _agent_id: AgentId, _text: &str) -> Result<(), TerminalRuntimeError> {
        Err(TerminalRuntimeError::NotRunning(self.running_for))
    }

    fn resize(
        &self,
        _agent_id: AgentId,
        _size: TerminalSize,
    ) -> Result<bool, TerminalRuntimeError> {
        Ok(false)
    }

    fn stop(&self, _agent_id: AgentId, _grace: Duration) -> Result<(), TerminalRuntimeError> {
        Ok(())
    }

    fn is_running(&self, agent_id: AgentId) -> bool {
        agent_id == self.running_for
    }

    fn current_run(&self, _agent_id: AgentId) -> Option<RunId> {
        Some(RunId(1))
    }
}

/// Keeps every entry the journal records.
#[derive(Default)]
struct RecordingRecorder(Mutex<Vec<AgentActionEntry>>);

impl RecordingRecorder {
    fn entries(&self) -> Vec<AgentActionEntry> {
        locked(&self.0).clone()
    }
}

impl ActionRecorder for RecordingRecorder {
    fn record(&self, entry: &AgentActionEntry) -> Result<(), PortError> {
        locked(&self.0).push(entry.clone());
        Ok(())
    }
}

fn journal_of(recorder: &Arc<RecordingRecorder>) -> DirectiveJournalBridge {
    DirectiveJournalBridge::new(
        ActionJournal::new(recorder.clone(), Arc::new(SystemClock)),
        StorageAlerts::new(test_log()),
    )
}

fn test_log() -> crate::bootstrap::ServerLog {
    let folder = tempfile::tempdir().expect("folder");
    let config = crate::config::Config {
        data_folder: folder.path().join("office"),
    };
    crate::bootstrap::open_log(&config).expect("log")
}

#[test]
fn is_running_follows_the_stored_status() {
    let agent_id = AgentId::generate();
    let bridge = DirectiveTargetBridge::new(Arc::new(OneAgent(agent_with_status(
        agent_id,
        "Ana",
        AgentStatus::WaitingForInput,
    ))));

    assert_eq!(bridge.is_running(agent_id), Ok(true));
    assert_eq!(bridge.is_running(AgentId::generate()), Ok(false));
}

#[test]
fn a_stopped_or_exited_member_is_not_running() {
    let agent_id = AgentId::generate();
    let bridge = DirectiveTargetBridge::new(Arc::new(OneAgent(agent_with_status(
        agent_id,
        "Ana",
        AgentStatus::Stopped,
    ))));

    assert_eq!(bridge.is_running(agent_id), Ok(false));
}

/// A store that cannot read: the bridge surfaces its failure instead of
/// dressing it as a stopped member (Efe r2 LOW).
struct BrokenStore;

impl AgentStore for BrokenStore {
    fn insert(&self, _agent: &Agent) -> Result<(), PortError> {
        Ok(())
    }

    fn find(&self, _id: AgentId) -> Result<Option<Agent>, PortError> {
        Err(PortError::Storage {
            message: "the agent store cannot be read".to_owned(),
            is_disk_failure: false,
        })
    }

    fn list(&self) -> Result<Vec<Agent>, PortError> {
        Err(PortError::Storage {
            message: "the agent store cannot be read".to_owned(),
            is_disk_failure: false,
        })
    }

    fn update_status(&self, _id: AgentId, _status: AgentStatus) -> Result<(), PortError> {
        Ok(())
    }

    fn delete(&self, _id: AgentId) -> Result<(), PortError> {
        Ok(())
    }
}

#[test]
fn a_store_failure_in_is_running_is_the_error_not_a_no() {
    let bridge = DirectiveTargetBridge::new(Arc::new(BrokenStore));

    let running = bridge.is_running(AgentId::generate());
    assert_eq!(
        running.as_ref().err().map(String::as_str),
        Some("storage failure: the agent store cannot be read"),
        "{running:?}"
    );
}

#[test]
fn name_of_names_the_agent_and_none_for_an_unknown_store() {
    let agent_id = AgentId::generate();
    let bridge = DirectiveTargetBridge::new(Arc::new(OneAgent(agent_with_status(
        agent_id,
        "Ana",
        AgentStatus::Working,
    ))));

    assert_eq!(bridge.name_of(agent_id).as_deref(), Some("Ana"));
    assert_eq!(bridge.name_of(AgentId::generate()), None);
}

#[test]
fn the_writer_delivers_text_and_enter_in_one_write() {
    let agent_id = AgentId::generate();
    let runtime = Arc::new(RecordingRuntime {
        written: Mutex::new(Vec::new()),
        running_for: agent_id,
    });
    let writer = DirectiveWriterBridge::new(runtime.clone());

    writer.write(agent_id, "ship it").expect("write");

    let written = locked(&runtime.written);
    assert_eq!(written.len(), 1);
    assert_eq!(written[0], (agent_id, b"ship it\r".to_vec()));
}

#[test]
fn a_control_character_in_a_refused_text_is_escaped_in_the_journal() {
    let sender = AgentId::generate();
    let target = AgentId::generate();
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = journal_of(&recorder);

    journal.record(sender, target, "bell \u{7} inside", Some("refused"));

    let entries = recorder.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].actor, kernel::Actor::Agent(sender));
    assert_eq!(entries[0].agent_id, target);
    assert_eq!(entries[0].action, AgentAction::Directive);
    assert_eq!(
        entries[0].failure.as_deref(),
        Some("refused: bell \\u0007 inside")
    );
}

#[test]
fn a_nine_thousand_character_text_is_capped_at_the_bound_with_a_hash() {
    let sender = AgentId::generate();
    let target = AgentId::generate();
    let text = "a".repeat(9000);
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = journal_of(&recorder);

    journal.record(sender, target, &text, Some("too long"));

    let entries = recorder.entries();
    let stored = entries[0]
        .failure
        .as_deref()
        .expect("the refusal is journaled with its text");
    // The whole failure text, reason included, is one capped unit.
    assert_eq!(stored.chars().count(), JOURNAL_MAX_CHARS);
    assert!(stored.contains('…'));
    // The hash is over the whole escaped failure text, reason included.
    let full = format!("too long: {text}");
    assert!(stored.ends_with(&text_hash(&full)), "{stored}");
    // The prefix is the escaped whole text's head, reason first.
    assert!(stored.starts_with(&format!("too long: {}", "a".repeat(PREFIX_CHARS - 10))));
}

#[test]
fn a_text_within_the_bound_is_journaled_whole() {
    let sender = AgentId::generate();
    let target = AgentId::generate();
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = journal_of(&recorder);

    journal.record(sender, target, "plain text", None);

    let entry = &recorder.entries()[0];
    assert_eq!(entry.failure, None);
    assert_eq!(entry.action, AgentAction::Directive);
}

#[test]
fn a_gateway_refusal_journals_the_caller_as_actor_and_agent() {
    let caller = AgentId::generate();
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = journal_of(&recorder);

    journal.refused(caller, "an agent may only act within its subtree");

    let entry = &recorder.entries()[0];
    assert_eq!(entry.actor, kernel::Actor::Agent(caller));
    assert_eq!(entry.agent_id, caller);
    assert_eq!(entry.action, AgentAction::Refused);
    assert_eq!(
        entry.failure.as_deref(),
        Some("an agent may only act within its subtree")
    );
}

#[test]
fn a_control_character_in_a_gateway_refusal_is_escaped_in_the_journal() {
    let caller = AgentId::generate();
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = journal_of(&recorder);

    journal.refused(caller, "bad field \u{7} here");

    let entry = &recorder.entries()[0];
    assert_eq!(entry.action, AgentAction::Refused);
    assert_eq!(
        entry.failure.as_deref(),
        Some("bad field \\u0007 here"),
        "the reason travels the same escape as a refused attempt"
    );
}

#[test]
fn a_gateway_refusal_over_the_bound_is_capped_with_a_hash() {
    let caller = AgentId::generate();
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = journal_of(&recorder);
    let reason = format!(
        "directive.send does not take the field {}",
        "k".repeat(9000)
    );

    journal.refused(caller, &reason);

    let stored = recorder.entries()[0]
        .failure
        .clone()
        .expect("the refusal is journaled");
    // The whole reason, however hostile, is one capped unit at the record's
    // own bound, the same as a refused attempt.
    assert_eq!(stored.chars().count(), JOURNAL_MAX_CHARS);
    assert!(stored.contains('…'));
    assert!(stored.ends_with(&text_hash(&escape_control_characters(&reason))));
}

#[test]
fn a_journal_write_failure_is_an_alert_and_not_a_panic() {
    struct FailingRecorder;
    impl ActionRecorder for FailingRecorder {
        fn record(&self, _entry: &AgentActionEntry) -> Result<(), PortError> {
            Err(PortError::Storage {
                message: "disk full".to_owned(),
                is_disk_failure: true,
            })
        }
    }

    let sender = AgentId::generate();
    let target = AgentId::generate();
    let journal = DirectiveJournalBridge::new(
        ActionJournal::new(Arc::new(FailingRecorder), Arc::new(SystemClock)),
        StorageAlerts::new(test_log()),
    );

    // The bridge owns the failure: the record write failing must not turn a
    // refusal into a panic of the request thread.
    journal.record(sender, target, "text", Some("refused"));
}

#[test]
fn a_refusal_of_an_empty_text_keeps_the_colon_in_the_journal() {
    let sender = AgentId::generate();
    let target = AgentId::generate();
    let recorder = Arc::new(RecordingRecorder::default());
    let journal = journal_of(&recorder);

    // The use case refused an empty directive: the record shows the reason
    // and the empty text it refused, so the audit sees what was sent (r2
    // LOW: the colon went missing and read as a reason without a text).
    journal.record(sender, target, "", Some("text must not be empty"));

    let entry = &recorder.entries()[0];
    assert_eq!(
        entry.failure.as_deref(),
        Some("text must not be empty: "),
        "{entry:?}"
    );
}
