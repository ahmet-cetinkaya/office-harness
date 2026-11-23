// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use agent_application::common::action_journal::ActionJournal;
use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::{AgentStore, Clock};
use agent_application::common::models::AgentAction;
use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{TaskSource, TaskText};
use agent_presentation::agent_task_event;
use hierarchy_application::common::ports::{
    Delivery, DirectiveJournal, DirectiveReceipt, DirectiveTarget, DirectiveWriter, Readiness,
    ReportJournal,
};
use hierarchy_domain::DIRECTIVE_MAX_CHARS;
use kernel::{Actor, AgentId};
use sha2::{Digest, Sha256};
use terminal_application::common::interfaces::TerminalRuntime;
use terminal_domain::services::{shows_open_question, shows_typed_text};
use terminal_domain::value_objects::ScreenView;

use super::{PersonInput, SleepingAgents, SystemClock};
use crate::bootstrap::StorageAlerts;
use crate::server::EventHub;

/// The longest text the journal keeps: the directive bound itself, so the
/// journal never holds more than a directive may carry.
const JOURNAL_MAX_CHARS: usize = DIRECTIVE_MAX_CHARS;

/// The hex digits of the hash that names what a capped text dropped.
const HASH_HEX_CHARS: usize = 16;

/// How many characters a capped text keeps before the ellipsis and the hash;
/// the one is the ellipsis itself.
const PREFIX_CHARS: usize = JOURNAL_MAX_CHARS - 1 - HASH_HEX_CHARS;

/// How long a typed directive may take to show at the input line before the
/// office gives up on its Enter (0012/FR-003).
const INPUT_LINE_WAIT: Duration = Duration::from_secs(2);

/// How often the screen is read while the office waits for the text.
const INPUT_LINE_POLL: Duration = Duration::from_millis(50);

const ENTER: &[u8] = b"\r";

/// Answers the hierarchy's target port: whether the member's session runs,
/// the name a refusal shows, and whether a directive may be typed now
/// (0012/FR-002) from the stored status, the person input, and the screen.
pub struct DirectiveTargetBridge {
    agents: Arc<dyn AgentStore>,
    runtime: Arc<dyn TerminalRuntime>,
    person_input: PersonInput,
    sleeping: SleepingAgents,
}

impl DirectiveTargetBridge {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        runtime: Arc<dyn TerminalRuntime>,
        person_input: PersonInput,
    ) -> Self {
        Self {
            agents,
            runtime,
            person_input,
            sleeping: SleepingAgents::default(),
        }
    }

    /// The agents going to sleep, which count as asleep and not running
    /// (0018/FR-005).
    #[must_use]
    pub fn with_sleeping(mut self, sleeping: SleepingAgents) -> Self {
        self.sleeping = sleeping;
        self
    }

    /// A screen the office cannot read counts as busy: it cannot tell that
    /// no question shows.
    fn screen_is_clear(&self, agent_id: AgentId) -> bool {
        self.runtime
            .screen(agent_id)
            .is_some_and(|screen| !shows_open_question(&screen))
    }
}

impl DirectiveTarget for DirectiveTargetBridge {
    fn is_running(&self, agent_id: AgentId) -> Result<bool, String> {
        // A store failure is the error it is (Efe r2 LOW): the caller must
        // not read a broken store as a stopped member.
        self.agents
            .find(agent_id)
            .map(|found| {
                found.is_some_and(|agent| {
                    agent.status.is_running() && !self.sleeping.contains(agent_id)
                })
            })
            .map_err(|error| error.to_string())
    }

    fn is_asleep(&self, agent_id: AgentId) -> bool {
        if self.sleeping.contains(agent_id) {
            return true;
        }
        self.agents
            .find(agent_id)
            .ok()
            .flatten()
            .is_some_and(|agent| agent.status == AgentStatus::Asleep)
    }

    fn name_of(&self, agent_id: AgentId) -> Option<String> {
        self.agents
            .find(agent_id)
            .ok()
            .flatten()
            .map(|agent| agent.name.to_string())
    }

    fn readiness(&self, agent_id: AgentId) -> Result<Readiness, String> {
        let status = self
            .agents
            .find(agent_id)
            .map_err(|error| error.to_string())?
            .map(|agent| agent.status);
        let Some(status) = status.filter(|status| status.is_running()) else {
            return Ok(Readiness::Gone);
        };
        let held_by_person = self
            .person_input
            .holds_directives(agent_id, self.runtime.current_run(agent_id));
        if status != AgentStatus::WaitingForInput || held_by_person {
            return Ok(Readiness::Busy);
        }
        Ok(if self.screen_is_clear(agent_id) {
            Readiness::Ready
        } else {
            Readiness::Busy
        })
    }
}

/// Delivers one directive: the text as one paste, marked as a paste when
/// the CLI asked for that, then the Enter only once the text, or a paste
/// placeholder, shows at the input line (0012/FR-003). Text that got no
/// Enter holds the target's next directives until a person types there
/// (0012 A11). The member still receives the text as one input (SC-006).
pub struct DirectiveWriterBridge {
    runtime: Arc<dyn TerminalRuntime>,
    person_input: PersonInput,
    input_line_wait: Duration,
}

impl DirectiveWriterBridge {
    #[must_use]
    pub fn new(runtime: Arc<dyn TerminalRuntime>, person_input: PersonInput) -> Self {
        Self {
            runtime,
            person_input,
            input_line_wait: INPUT_LINE_WAIT,
        }
    }

    /// Reads the screen until the typed text shows or the wait ends.
    fn text_showed(&self, agent_id: AgentId, before: &ScreenView, text: &str) -> bool {
        let deadline = Instant::now() + self.input_line_wait;
        loop {
            let shows = self
                .runtime
                .screen(agent_id)
                .is_some_and(|after| shows_typed_text(before, &after, text));
            if shows || Instant::now() >= deadline {
                return shows;
            }
            thread::sleep(INPUT_LINE_POLL);
        }
    }
}

impl DirectiveWriter for DirectiveWriterBridge {
    fn deliver(&self, agent_id: AgentId, text: &str) -> Result<Delivery, String> {
        let before = self.runtime.screen(agent_id).unwrap_or_default();
        let run = self.runtime.current_run(agent_id);
        // Stamped before the paste, so a sign-in watch never reads the
        // directive as the CLI's own screen (0019 A5).
        self.person_input.directive_typed(agent_id);
        self.runtime
            .paste(agent_id, text)
            .map_err(|error| error.to_string())?;
        if self.text_showed(agent_id, &before, text) {
            self.runtime
                .write_input(agent_id, ENTER)
                .map_err(|error| error.to_string())?;
            return Ok(Delivery::Submitted);
        }
        if let Some(run) = run {
            self.person_input.office_text_left(agent_id, run);
        }
        Ok(Delivery::Unsubmitted)
    }
}

/// Held while a task text is stored and its `agent.task` event is published,
/// so that clients see the events in the order the texts were stored (0011).
///
/// Lock order: `write_order` (the directive handler) or `StatusOrder` (the
/// agent gateway) comes first, and this lock is always the innermost. Nothing
/// that takes either of those runs while this one is held.
pub type TaskOrder = Arc<Mutex<()>>;

/// Keeps the text of a received directive as the target's task text (0011,
/// FR-002) and publishes `agent.task` to every client. The time is the
/// server clock at receipt. A failed store write is the bridge's to report:
/// the directive is already in the terminal, so it still answers Ok.
pub struct DirectiveReceiptBridge {
    agents: Arc<dyn AgentStore>,
    events: EventHub,
    alerts: StorageAlerts,
    task_order: TaskOrder,
}

impl DirectiveReceiptBridge {
    #[must_use]
    pub fn new(
        agents: Arc<dyn AgentStore>,
        events: EventHub,
        alerts: StorageAlerts,
        task_order: TaskOrder,
    ) -> Self {
        Self {
            agents,
            events,
            alerts,
            task_order,
        }
    }
}

impl DirectiveReceipt for DirectiveReceiptBridge {
    fn received(&self, target: AgentId, text: &str) {
        // The send handler already refused blank and over-long texts, so a
        // text that gives no task here has nothing to keep.
        let Ok(Some(task)) = TaskText::from_input(text, SystemClock.now(), TaskSource::Directive)
        else {
            return;
        };
        let _in_order = self
            .task_order
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match self.agents.update_task(target, Some(&task)) {
            Ok(()) => self.events.publish(&agent_task_event(target, Some(&task))),
            Err(error) => self
                .alerts
                .report_disk_failure(&format!("the task text failed: {error}")),
        }
    }
}

/// Writes the directive attempts into the action record under the sender's
/// actor (a refusal in the sender's record and the target's, an accepted
/// directive in the target's), and owns a failed record write (the port's
/// contract). A refusal keeps the text as it was sent, escaped and capped,
/// so a hostile text can carry no control character into the record and
/// cannot flood it (Efe r2).
#[derive(Clone)]
pub struct DirectiveJournalBridge {
    journal: ActionJournal,
    alerts: StorageAlerts,
}

impl DirectiveJournalBridge {
    #[must_use]
    pub fn new(journal: ActionJournal, alerts: StorageAlerts) -> Self {
        Self { journal, alerts }
    }

    /// Records a request the agent-call gateway refused: the caller is the
    /// actor and the named agent, because no other agent is involved
    /// (FR-009). The reason may carry client text, so it goes through the
    /// same escape and cap as a refused attempt (Efe, PR 35 MEDIUM-1).
    pub fn refused(&self, caller: AgentId, reason: &str) {
        self.record_entry(
            caller,
            caller,
            AgentAction::Refused,
            Some(&refused_attempt(reason, None)),
        );
    }

    fn record_entry(
        &self,
        actor: AgentId,
        target: AgentId,
        action: AgentAction,
        failure: Option<&str>,
    ) {
        self.record_detailed_entry(actor, target, action, failure, None);
    }

    fn record_detailed_entry(
        &self,
        actor: AgentId,
        target: AgentId,
        action: AgentAction,
        failure: Option<&str>,
        detail: Option<&str>,
    ) {
        self.record_as(Actor::Agent(actor), target, action, failure, detail);
    }

    fn record_as(
        &self,
        actor: Actor,
        target: AgentId,
        action: AgentAction,
        failure: Option<&str>,
        detail: Option<&str>,
    ) {
        // The use cases share one journal and one request runs on one
        // thread, so the scope guard, not a parameter, names the actor.
        let _actor = ActorScope::enter(actor);
        if let Err(error) = self
            .journal
            .record_detailed(target, action, failure, detail)
        {
            self.alerts
                .report_disk_failure(&format!("the action record failed: {error}"));
        }
    }
}

impl DirectiveJournal for DirectiveJournalBridge {
    fn record(&self, sender: AgentId, target: AgentId, text: &str, failure: Option<&str>) {
        let journaled = failure.map(|reason| refused_attempt(reason, Some(text)));
        self.record_entry(sender, target, AgentAction::Directive, journaled.as_deref());
        if journaled.is_some() && sender != target {
            // The owner decided that a refusal shows in the sender's record
            // too (ACF-168): the sender asked for it, and its details must
            // show what the office refused. An accepted directive stays
            // on the target's record only.
            self.record_entry(sender, sender, AgentAction::Directive, journaled.as_deref());
        }
    }

    fn record_objective(&self, creator: Actor, temp: AgentId) {
        self.record_as(creator, temp, AgentAction::Directive, None, None);
    }
}

/// Reports go into the same record as directives (0015/FR-009): an accepted
/// one under its receiver with its kind and visible text as the detail, a
/// refused one, with the text as sent, under the sender and the parent.
impl ReportJournal for DirectiveJournalBridge {
    fn accepted(&self, sender: AgentId, receiver: AgentId, record_text: &str) {
        let detail = capped_at_journal_bound(&escape_control_characters(record_text));
        self.record_detailed_entry(sender, receiver, AgentAction::Report, None, Some(&detail));
    }

    fn refused(&self, sender: AgentId, parent: Option<AgentId>, text: &str, reason: &str) {
        let journaled = refused_attempt(reason, Some(text));
        self.record_entry(sender, sender, AgentAction::Report, Some(&journaled));
        if let Some(parent) = parent.filter(|parent| *parent != sender) {
            self.record_entry(sender, parent, AgentAction::Report, Some(&journaled));
        }
    }
}

/// The journal form of a refused attempt (FR-011): the reason, then the text
/// as it was sent (an empty text stays an empty text after the colon, so a
/// refusal of an empty directive shows that the text was empty; a refusal of
/// the gateway itself carries no directive text and is called without one),
/// so the audit shows what was attempted. Both the reason and the text may
/// carry client words, so both are escaped, and the whole failure text is
/// capped as one, so a long reason cannot push the representation past the
/// bound either. Past the cap the text keeps its first characters, one
/// ellipsis, and the hash of the whole.
fn refused_attempt(reason: &str, text: Option<&str>) -> String {
    let escaped = match text {
        None => escape_control_characters(reason),
        Some(text) => format!("{reason}: {}", escape_control_characters(text)),
    };
    capped_at_journal_bound(&escaped)
}

/// The record keeps at most `JOURNAL_MAX_CHARS` of one failure text: past
/// the bound it keeps the first characters, one ellipsis, and the hash of
/// the whole.
fn capped_at_journal_bound(escaped: &str) -> String {
    if escaped.chars().count() <= JOURNAL_MAX_CHARS {
        return escaped.to_owned();
    }
    let mut capped: String = escaped.chars().take(PREFIX_CHARS).collect();
    capped.push('…');
    capped.push_str(&text_hash(escaped));
    capped
}

/// Control characters cannot travel the record as themselves, so each
/// becomes its `\u00xx` escape; `escape_debug` would also quote and shorten,
/// which the record must not show.
fn escape_control_characters(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_control() {
            let _ = write!(escaped, "\\u{:04x}", character as u32);
        } else {
            escaped.push(character);
        }
    }
    escaped
}

/// The first hex digits of SHA-256 over the whole text: short, but no two
/// everyday texts share it.
fn text_hash(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut hex = String::with_capacity(HASH_HEX_CHARS);
    for byte in digest.iter().take(HASH_HEX_CHARS / 2) {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
#[path = "directive_bridges.tests.rs"]
mod tests;
