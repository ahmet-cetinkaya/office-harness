// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::{Actor, AgentId, ScheduleId, Timestamp};

/// Whether the office may type a directive into a target now (0012/FR-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// The target waits for input, no person is typing there, no office
    /// text waits for the person, and no open question shows.
    Ready,
    /// The target runs, but one of those conditions does not hold yet.
    Busy,
    /// The target's session is no longer running.
    Gone,
}

/// What one delivery did (0012/FR-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// The text showed at the input line and the Enter followed.
    Submitted,
    /// The text was typed, but it did not show at the input line, so no
    /// Enter followed.
    Unsubmitted,
}

/// Whether a target agent's session can take input right now. A store
/// failure is its own error: a silent "not running" would
/// dress a broken store as a stopped member.
pub trait DirectiveTarget: Send + Sync {
    /// # Errors
    /// The store's own failure when the target cannot be read.
    fn is_running(&self, agent_id: AgentId) -> Result<bool, String>;

    /// Whether the agent is asleep or going to sleep (0018), so a refusal can
    /// say "asleep" instead of "not running". A store failure reads as not
    /// asleep: the plain not-running reason is still true.
    #[must_use]
    fn is_asleep(&self, agent_id: AgentId) -> bool;

    /// The agent's name as the office shows it, so a refusal can name the
    /// agent instead of its id. `None` when the id cannot be resolved, for
    /// example when the agent is deleted between the tree snapshot and this
    /// lookup.
    #[must_use]
    fn name_of(&self, agent_id: AgentId) -> Option<String>;

    /// Whether the agent is a temporary agent (0020), so a report it sends
    /// outlives its removal in the queue. A host with no temps answers false.
    #[must_use]
    fn is_temp(&self, _agent_id: AgentId) -> bool {
        false
    }

    /// Whether the office may type into the target now (0012/FR-002).
    ///
    /// # Errors
    /// The store's own failure when the target cannot be read; the caller
    /// then keeps the directive waiting.
    fn readiness(&self, agent_id: AgentId) -> Result<Readiness, String>;
}

/// Delivers one directive to a target agent's session: the text as one
/// paste, then the Enter only once the text shows at the input line
/// (0012/FR-003).
pub trait DirectiveWriter: Send + Sync {
    /// # Errors
    /// The host's message when the text could not be written.
    fn deliver(&self, agent_id: AgentId, text: &str) -> Result<Delivery, String>;
}

/// The time the queue measures waits and gaps with: one that a step of the
/// system clock never moves (0012/FR-005, the 10-minute limit).
pub trait DirectiveClock: Send + Sync {
    #[must_use]
    fn steady_now(&self) -> Timestamp;
}

/// Keeps the directive attempts in the action record (FR-009, FR-011). It is
/// a port so this crate stays independent of the agent module's journal; the
/// bridge journals inside the sender's `ActorScope` and owns any record-write
/// failure.
pub trait DirectiveJournal: Send + Sync {
    /// Records one attempt together with the text as it was sent, refused or
    /// not: the audit must show what was attempted, not only that something
    /// was (FR-011). The bridge stores the full text and its hash as it does
    /// for the 0001 actions, and a refusal keeps the rejected text as sent.
    fn record(&self, sender: AgentId, target: AgentId, text: &str, failure: Option<&str>);

    /// Records the objective of a temporary agent once, in the temp's own
    /// record, with the creator as actor (0020/FR-001).
    fn record_objective(&self, creator: Actor, temp: AgentId);

    /// Records what went wrong with a scheduled directive once, in the
    /// target's record, with the schedule as actor: no record of a sender
    /// exists (0025/FR-012). `failure` is the unsubmitted text or the
    /// withdrawal with its reason. A host that keeps no schedule records
    /// keeps the default, which writes nothing.
    fn record_scheduled(
        &self,
        _schedule: ScheduleId,
        _target: AgentId,
        _text: &str,
        _failure: &str,
    ) {
    }
}

/// How the queue settled a scheduled directive that had waited (0025/FR-009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduledOutcome {
    Submitted,
    Unsubmitted(String),
    Withdrawn(String),
}

/// Tells the schedule module what became of a scheduled directive that
/// waited in a queue, keyed by the run it belongs to. The handler calls it
/// once per entry, after the entry left the queue.
pub trait ScheduledOutcomes: Send + Sync {
    fn settled(&self, run_id: i64, outcome: ScheduledOutcome);
}

/// Tells the office that a target received a directive: the text is in its
/// terminal. The office keeps it as the target's task text (0011). The
/// bridge stamps the time and owns any failure, so a failed store never
/// turns a delivered directive into an error.
pub trait DirectiveReceipt: Send + Sync {
    fn received(&self, target: AgentId, text: &str);
}
