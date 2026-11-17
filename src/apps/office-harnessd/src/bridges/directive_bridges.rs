// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write as _;
use std::sync::Arc;

use agent_application::common::action_journal::ActionJournal;
use agent_application::common::actor_scope::ActorScope;
use agent_application::common::interfaces::AgentStore;
use agent_application::common::models::AgentAction;
use hierarchy_application::common::ports::{DirectiveJournal, DirectiveTarget, DirectiveWriter};
use hierarchy_domain::DIRECTIVE_MAX_CHARS;
use kernel::{Actor, AgentId};
use sha2::{Digest, Sha256};
use terminal_application::common::interfaces::TerminalRuntime;

use crate::bootstrap::StorageAlerts;

/// The longest text the journal keeps: the directive bound itself, so the
/// journal never holds more than a directive may carry.
const JOURNAL_MAX_CHARS: usize = DIRECTIVE_MAX_CHARS;

/// The hex digits of the hash that names what a capped text dropped.
const HASH_HEX_CHARS: usize = 16;

/// How many characters a capped text keeps before the ellipsis and the hash;
/// the one is the ellipsis itself.
const PREFIX_CHARS: usize = JOURNAL_MAX_CHARS - 1 - HASH_HEX_CHARS;

/// Answers the hierarchy's target port from the agent store: whether the
/// member's session can take input, and the name a refusal shows. A store
/// failure reads as not running, because the write itself reports its own
/// failure to the caller.
pub struct DirectiveTargetBridge {
    agents: Arc<dyn AgentStore>,
}

impl DirectiveTargetBridge {
    #[must_use]
    pub fn new(agents: Arc<dyn AgentStore>) -> Self {
        Self { agents }
    }
}

impl DirectiveTarget for DirectiveTargetBridge {
    fn is_running(&self, agent_id: AgentId) -> bool {
        self.agents
            .find(agent_id)
            .ok()
            .flatten()
            .is_some_and(|agent| agent.status.is_running())
    }

    fn name_of(&self, agent_id: AgentId) -> Option<String> {
        self.agents
            .find(agent_id)
            .ok()
            .flatten()
            .map(|agent| agent.name.to_string())
    }
}

/// Delivers one directive with one `write_input` of the text and the Enter
/// (SC-006). The bridge appends the Enter, so the use case stays
/// terminal-free and the member receives the text as one input.
pub struct DirectiveWriterBridge {
    runtime: Arc<dyn TerminalRuntime>,
}

impl DirectiveWriterBridge {
    #[must_use]
    pub fn new(runtime: Arc<dyn TerminalRuntime>) -> Self {
        Self { runtime }
    }
}

impl DirectiveWriter for DirectiveWriterBridge {
    fn write(&self, agent_id: AgentId, text: &str) -> Result<(), String> {
        let typed = format!("{text}\r");
        self.runtime
            .write_input(agent_id, typed.as_bytes())
            .map_err(|error| error.to_string())
    }
}

/// Writes the directive attempts into the action record under the sender's
/// actor, and owns a failed record write (the port's contract). A refusal
/// keeps the text as it was sent, escaped and capped, so a hostile text can
/// carry no control character into the record and cannot flood it (Efe r2).
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
    /// (FR-009).
    pub fn refused(&self, caller: AgentId, reason: &str) {
        self.record_entry(caller, caller, AgentAction::Refused, Some(reason));
    }

    fn record_entry(
        &self,
        actor: AgentId,
        target: AgentId,
        action: AgentAction,
        failure: Option<&str>,
    ) {
        // The use cases share one journal and one request runs on one
        // thread, so the scope guard, not a parameter, names the actor.
        let _actor = ActorScope::enter(Actor::Agent(actor));
        if let Err(error) = self.journal.record(target, action, failure) {
            self.alerts
                .report_disk_failure(&format!("the action record failed: {error}"));
        }
    }
}

impl DirectiveJournal for DirectiveJournalBridge {
    fn record(&self, sender: AgentId, target: AgentId, text: &str, failure: Option<&str>) {
        let journaled = failure.map(|reason| refused_attempt(reason, text));
        self.record_entry(sender, target, AgentAction::Directive, journaled.as_deref());
    }
}

/// The journal form of a refused attempt (FR-011): the reason, then the text
/// as it was sent, so the audit shows what was attempted. Control characters
/// become their `\u00xx` escape, since they cannot sit in the record as
/// themselves, and the whole failure text is capped as one, so a long reason
/// cannot push the representation past the bound either. Past the cap the
/// text keeps its first characters, one ellipsis, and the hash of the whole.
fn refused_attempt(reason: &str, text: &str) -> String {
    let escaped = format!("{reason}: {}", escape_control_characters(text));
    if escaped.chars().count() <= JOURNAL_MAX_CHARS {
        return escaped;
    }
    let mut capped: String = escaped.chars().take(PREFIX_CHARS).collect();
    capped.push('…');
    capped.push_str(&text_hash(&escaped));
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
