// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use hierarchy_domain::{AgentAction, HierarchyError, check_descendant, validate_directive_text};
use kernel::AgentId;

use super::directive_queue::{DirectiveQueue, QUEUE_LIMIT, WaitingDirective};
use crate::commands::hold::HoldService;
use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;
use crate::common::ports::{
    Delivery, DirectiveClock, DirectiveJournal, DirectiveReceipt, DirectiveTarget, DirectiveWriter,
    Readiness,
};

/// The record text of a directive that was typed but got no Enter
/// (0012/FR-003).
const UNSUBMITTED: &str = "typed but not submitted: the text did not show at the input line";

/// The reason a directive that waited too long is withdrawn (0012/FR-005).
const EXPIRED: &str = "not typed within 10 minutes";

/// The refusal of a directive sent after a server stop withdrew the waiting
/// ones (0012/FR-005).
const STOPPING: &str = "the office server is stopping";

/// What became of an accepted directive (0012/FR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveOutcome {
    /// Typed and followed by Enter.
    Submitted,
    /// In the target's queue, at this place counted from 1.
    Waiting { position: usize },
    /// Typed, but no Enter followed: the text did not show at the input line.
    Unsubmitted,
}

/// Whether a directive that waited may still be typed: the tree and the
/// holds can change while it waits (0012/FR-005).
enum Recheck {
    Allowed,
    Refused(String),
    /// The tree cannot be read; the directive keeps waiting.
    Unreadable,
}

/// Sends one directive from `sender` to one of its descendants (FR-009,
/// FR-011). The target must sit in the sender's subtree, the text must pass
/// the directive rules, and the target's session must be running. The
/// directive is then typed at once when the target is ready (0012/FR-002),
/// or waits in the target's queue for `deliver_waiting`. Every refusal and
/// every withdrawal is journaled, so the action record keeps every attempt.
pub struct SendDirectiveHandler {
    store: Arc<dyn HierarchyStore>,
    targets: Arc<dyn DirectiveTarget>,
    writer: Arc<dyn DirectiveWriter>,
    journal: Arc<dyn DirectiveJournal>,
    receipt: Arc<dyn DirectiveReceipt>,
    clock: Arc<dyn DirectiveClock>,
    queue: DirectiveQueue,
    hold_guard: Option<Arc<HoldService>>,
}

/// The ports a `SendDirectiveHandler` talks to.
pub struct DirectivePorts {
    pub store: Arc<dyn HierarchyStore>,
    pub targets: Arc<dyn DirectiveTarget>,
    pub writer: Arc<dyn DirectiveWriter>,
    pub journal: Arc<dyn DirectiveJournal>,
    pub receipt: Arc<dyn DirectiveReceipt>,
    pub clock: Arc<dyn DirectiveClock>,
}

impl SendDirectiveHandler {
    #[must_use]
    pub fn new(ports: DirectivePorts) -> Self {
        let DirectivePorts {
            store,
            targets,
            writer,
            journal,
            receipt,
            clock,
        } = ports;
        Self {
            store,
            targets,
            writer,
            journal,
            receipt,
            clock,
            queue: DirectiveQueue::default(),
            hold_guard: None,
        }
    }

    /// Puts the hold and rate-limit gate in front of the terminal write.
    /// The gate journals no refusal; this handler journals each one with the
    /// target and the text (0007).
    #[must_use]
    pub fn with_hold_guard(mut self, hold_guard: Arc<HoldService>) -> Self {
        self.hold_guard = Some(hold_guard);
        self
    }

    /// # Errors
    /// `Storage` when the tree cannot be read; `Invalid` naming the rule when
    /// the target is not a descendant, the text breaks the directive rules,
    /// or the target is not running; `Refused` with the reason when the
    /// target's queue is full, a server stop has begun, or a hold or a rate
    /// limit stops it;
    /// `Conflict` carrying the writer's own message when the text cannot be
    /// written.
    pub fn handle(
        &self,
        sender: AgentId,
        target: AgentId,
        text: &str,
    ) -> Result<DirectiveOutcome, HierarchyApplicationError> {
        let snapshot = self
            .store
            .snapshot()
            .map_err(|error| self.journaled(sender, target, text, error))?;
        if let Err(error) = check_descendant(&snapshot.rows, sender, target) {
            return Err(self.journaled(sender, target, text, error.into()));
        }
        if let Err(error) = validate_directive_text(text) {
            return Err(self.journaled(sender, target, text, error.into()));
        }
        // A store failure behind the lookup is `Storage`, its own message:
        // the target may run, but the office cannot say so (Efe r2 LOW).
        if !self
            .targets
            .is_running(target)
            .map_err(HierarchyApplicationError::Storage)?
        {
            let name = self.name_of(target);
            return Err(self.journaled(
                sender,
                target,
                text,
                HierarchyApplicationError::Invalid(HierarchyError::NotRunning { name }),
            ));
        }
        // Before the hold guard, so a full queue never takes a rate-window
        // slot or feeds the automatic hold (0012 A6, 0007/FR-008).
        if self.queue.waiting_count(target) >= QUEUE_LIMIT {
            let reason = format!(
                "the queue of {} is full: {QUEUE_LIMIT} directives wait",
                self.name_of(target)
            );
            return Err(self.journaled(
                sender,
                target,
                text,
                HierarchyApplicationError::Refused(reason),
            ));
        }
        if let Some(hold_guard) = &self.hold_guard {
            // After every rule above, so a directive the rules refuse or a
            // stopped target never counts toward a limit (spec A10). The
            // guard journals no refusal, so the record keeps the target and
            // the text as sent, like every other refusal here (FR-010).
            if let Err(refusal) = hold_guard.guard(
                sender,
                AgentAction::Directive { target },
                Some(target),
                &snapshot.rows,
            ) {
                return Err(self.journaled(
                    sender,
                    target,
                    text,
                    HierarchyApplicationError::Refused(refusal.reason),
                ));
            }
        }
        self.accept(sender, target, text)
    }

    /// Types waiting directives whose target is ready, one per target, and
    /// withdraws the ones that may no longer be typed (0012/FR-001, FR-005).
    /// A delivery thread calls it on a short period while directives wait.
    pub fn deliver_waiting(&self) {
        for target in self.queue.targets_with_waiting() {
            self.deliver_next(target);
        }
    }

    /// Withdraws every waiting directive with `reason` and refuses every
    /// later one, as a normal server stop does (0012/FR-005).
    pub fn withdraw_all(&self, reason: &str) {
        for directive in self.queue.close() {
            self.withdrawn(&directive, reason);
        }
    }

    /// Blocks until a directive waits or `timeout` passes, and says whether
    /// one waits.
    pub fn wait_for_waiting(&self, timeout: Duration) -> bool {
        self.queue.wait_for_work(timeout)
    }

    /// Types the directive now when nothing waits before it and the target
    /// is ready; otherwise puts it in the queue. The accepted directive is
    /// journaled once either way (PB-2).
    fn accept(
        &self,
        sender: AgentId,
        target: AgentId,
        text: &str,
    ) -> Result<DirectiveOutcome, HierarchyApplicationError> {
        if self.queue.claim_empty(target, self.clock.steady_now()) {
            if self.targets.readiness(target) == Ok(Readiness::Ready) {
                return self.type_accepted(sender, target, text);
            }
            self.queue.release(target);
        }
        let Some(position) = self.queue.push(WaitingDirective {
            sender,
            target,
            text: text.to_owned(),
            accepted_at: self.clock.steady_now(),
        }) else {
            return Err(self.journaled(
                sender,
                target,
                text,
                HierarchyApplicationError::Refused(STOPPING.to_owned()),
            ));
        };
        self.journal.record(sender, target, text, None);
        Ok(DirectiveOutcome::Waiting { position })
    }

    /// The fast path: types a directive that never waited.
    fn type_accepted(
        &self,
        sender: AgentId,
        target: AgentId,
        text: &str,
    ) -> Result<DirectiveOutcome, HierarchyApplicationError> {
        match self.type_claimed(target, text) {
            Ok(Delivery::Submitted) => {
                self.journal.record(sender, target, text, None);
                Ok(DirectiveOutcome::Submitted)
            }
            Ok(Delivery::Unsubmitted) => {
                self.journal.record(sender, target, text, None);
                self.journal.record(sender, target, text, Some(UNSUBMITTED));
                Ok(DirectiveOutcome::Unsubmitted)
            }
            Err(message) => Err(self.journaled(
                sender,
                target,
                text,
                HierarchyApplicationError::Conflict(message),
            )),
        }
    }

    /// Delivers under the target's claim and ends it. The receipt comes
    /// before the claim ends, so the order of deliveries to one target is
    /// the order of its receipts (0011 D2); only a submitted text is a
    /// received task (0012 PB-3).
    fn type_claimed(&self, target: AgentId, text: &str) -> Result<Delivery, String> {
        let delivery = self.writer.deliver(target, text);
        if delivery == Ok(Delivery::Submitted) {
            self.receipt.received(target, text);
        }
        self.queue.finish(target, self.clock.steady_now());
        delivery
    }

    /// One delivery round for one target: withdraws what expired, then
    /// types, withdraws, or keeps the head.
    fn deliver_next(&self, target: AgentId) {
        let claim = self.queue.claim_head(target, self.clock.steady_now());
        for directive in &claim.expired {
            self.withdrawn(directive, EXPIRED);
        }
        let Some(head) = claim.head else {
            return;
        };
        match self.targets.readiness(target) {
            Ok(Readiness::Ready) => self.deliver_head(&head),
            Ok(Readiness::Gone) => {
                self.queue.release(target);
                let reason = format!("{} is no longer running", self.name_of(target));
                for directive in self.queue.drain(target) {
                    self.withdrawn(&directive, &reason);
                }
            }
            Ok(Readiness::Busy) | Err(_) => self.queue.release(target),
        }
    }

    fn deliver_head(&self, head: &WaitingDirective) {
        let target = head.target;
        match self.recheck(head) {
            Recheck::Unreadable => self.queue.release(target),
            Recheck::Refused(reason) => {
                let taken = self.queue.take_head(head);
                self.queue.release(target);
                // Not taken: a server stop withdrew it and recorded why.
                if taken {
                    self.withdrawn(head, &reason);
                }
            }
            Recheck::Allowed => {
                // Not taken: a server stop withdrew it, so it stays untyped.
                if !self.queue.take_head(head) {
                    self.queue.release(target);
                    return;
                }
                match self.type_claimed(target, &head.text) {
                    Ok(Delivery::Submitted) => {}
                    Ok(Delivery::Unsubmitted) => {
                        self.journal
                            .record(head.sender, target, &head.text, Some(UNSUBMITTED));
                    }
                    Err(message) => {
                        self.withdrawn(head, &format!("the text could not be typed: {message}"));
                    }
                }
            }
        }
    }

    /// Runs the tree rule and the hold check again for a directive that
    /// waited. The hold check counts nothing (0012 A6).
    fn recheck(&self, head: &WaitingDirective) -> Recheck {
        let Ok(snapshot) = self.store.snapshot() else {
            return Recheck::Unreadable;
        };
        if let Err(error) = check_descendant(&snapshot.rows, head.sender, head.target) {
            return Recheck::Refused(error.to_string());
        }
        let hold_reason = self
            .hold_guard
            .as_ref()
            .and_then(|guard| guard.refusal(head.sender, head.target, &snapshot.rows));
        hold_reason.map_or(Recheck::Allowed, Recheck::Refused)
    }

    /// Records a directive that left the queue untyped, in the sender's and
    /// the target's records (0012/FR-005).
    fn withdrawn(&self, directive: &WaitingDirective, reason: &str) {
        self.journal.record(
            directive.sender,
            directive.target,
            &directive.text,
            Some(&format!("withdrawn before typing: {reason}")),
        );
    }

    fn name_of(&self, target: AgentId) -> String {
        self.targets
            .name_of(target)
            .unwrap_or_else(|| target.to_string())
    }

    /// Records the attempt before the caller sees its outcome (FR-011): the
    /// action record carries every refusal with the text as sent and the
    /// reason, so the audit shows what was attempted.
    fn journaled(
        &self,
        sender: AgentId,
        target: AgentId,
        text: &str,
        error: HierarchyApplicationError,
    ) -> HierarchyApplicationError {
        self.journal
            .record(sender, target, text, Some(&error.to_string()));
        error
    }
}

#[cfg(test)]
#[path = "send_directive_handler.tests.rs"]
mod tests;
