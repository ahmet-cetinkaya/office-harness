// SPDX-License-Identifier: AGPL-3.0-or-later

use kernel::AgentId;

/// Whether a target agent's session can take input right now. A store
/// failure is its own error (Efe r2 LOW): a silent "not running" would
/// dress a broken store as a stopped member.
pub trait DirectiveTarget: Send + Sync {
    /// # Errors
    /// The store's own failure when the target cannot be read.
    fn is_running(&self, agent_id: AgentId) -> Result<bool, String>;

    /// The agent's name as the office shows it, so a refusal can name the
    /// agent instead of its id. `None` when the id cannot be resolved, for
    /// example when the agent is deleted between the tree snapshot and this
    /// lookup.
    #[must_use]
    fn name_of(&self, agent_id: AgentId) -> Option<String>;
}

/// Delivers one directive to a target agent's session. One call must deliver
/// the text; the bridge appends the Enter and does one `write_input`.
pub trait DirectiveWriter: Send + Sync {
    /// # Errors
    /// The host's message when the text could not be written.
    fn write(&self, agent_id: AgentId, text: &str) -> Result<(), String>;
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
}
