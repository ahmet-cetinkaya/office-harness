// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use hierarchy_domain::{
    AgentAction, HierarchyError, ReportReceiver, TreeRow, check_descendant, report_receiver,
    validate_directive_text,
};
use kernel::AgentId;

use super::directive_queue::{
    DirectiveQueue, QUEUE_LIMIT, REPORT_SHARE, ReportPush, ReportRoom, WaitingDirective,
};
use crate::commands::hold::HoldService;
use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;
use crate::common::ports::{
    Delivery, DirectiveClock, DirectiveJournal, DirectiveReceipt, DirectiveTarget, DirectiveWriter,
    Readiness, ReportJournal,
};

/// Why a target that does not run refuses a directive or a report: asleep
/// (0018), or plainly not running.
pub(crate) fn not_running_error(
    targets: &dyn DirectiveTarget,
    target: AgentId,
    name: String,
) -> HierarchyError {
    if targets.is_asleep(target) {
        HierarchyError::Asleep { name }
    } else {
        HierarchyError::NotRunning { name }
    }
}

/// The record text of a directive that was typed but got no Enter
/// (0012/FR-003).
pub(crate) const UNSUBMITTED: &str =
    "typed but not submitted: the text did not show at the input line";

/// The reason a directive that waited too long is withdrawn (0012/FR-005).
const EXPIRED: &str = "not typed within 10 minutes";

/// The refusal of a directive sent after a server stop withdrew the waiting
/// ones (0012/FR-005).
const STOPPING: &str = "the office server is stopping";

/// Why a waiting report left the queue: its sender sent a newer one
/// (0015/FR-010).
const REPLACED: &str = "replaced by a newer report";

/// A report to a parent, ready for the parent's queue (0015/FR-004). `line`
/// is the heading and the text, `record_text` the kind and the text, both in
/// visible form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentReport {
    pub sender: AgentId,
    pub parent: AgentId,
    pub line: String,
    pub record_text: String,
}

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
    report_journal: Option<Arc<dyn ReportJournal>>,
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
            report_journal: None,
        }
    }

    /// Records the reports that share the queue (0015/FR-009): a report that
    /// is withdrawn, replaced, or left unsubmitted goes into the report
    /// journal, not the directive journal.
    #[must_use]
    pub fn with_report_journal(mut self, report_journal: Arc<dyn ReportJournal>) -> Self {
        self.report_journal = Some(report_journal);
        self
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
            let refusal = not_running_error(&*self.targets, target, self.name_of(target));
            return Err(self.journaled(
                sender,
                target,
                text,
                HierarchyApplicationError::Invalid(refusal),
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

    /// Puts a report in its parent's queue, or types it at once when nothing
    /// waits there and the parent is ready (0015/FR-004). Room comes first,
    /// so a refused report takes no rate-window slot (0015/FR-010), then the
    /// holds and the reports limit (FR-006). The caller has checked the text
    /// and that the parent runs, and journals the outcome; this journals
    /// only the report that a newer one replaces.
    ///
    /// # Errors
    /// `Refused` when the parent's queue has no room, a hold or the reports
    /// limit stops it, or a server stop has begun; `Conflict` carrying the
    /// writer's message when the line cannot be typed.
    pub fn send_report(
        &self,
        report: &ParentReport,
        rows: &[TreeRow],
    ) -> Result<DirectiveOutcome, HierarchyApplicationError> {
        let room = self.queue.report_room(report.parent, report.sender);
        self.check_report_room(report.parent, room)?;
        if let Some(hold_guard) = &self.hold_guard {
            hold_guard
                .guard(
                    report.sender,
                    AgentAction::Report,
                    Some(report.parent),
                    rows,
                )
                .map_err(|refusal| HierarchyApplicationError::Refused(refusal.reason))?;
        }
        if room == ReportRoom::Add
            && self
                .queue
                .claim_empty(report.parent, self.clock.steady_now())
        {
            if self.targets.readiness(report.parent) == Ok(Readiness::Ready) {
                return self.type_report_now(report);
            }
            self.queue.release(report.parent);
        }
        self.queue_report(report)
    }

    fn check_report_room(
        &self,
        parent: AgentId,
        room: ReportRoom,
    ) -> Result<(), HierarchyApplicationError> {
        match room {
            ReportRoom::Replace | ReportRoom::Add => Ok(()),
            ReportRoom::NoRoomForReports | ReportRoom::Full => Err(self.no_room(parent, room)),
        }
    }

    /// The refusal of a report that finds no room in its parent's queue.
    fn no_room(&self, parent: AgentId, room: ReportRoom) -> HierarchyApplicationError {
        let name = self.name_of(parent);
        HierarchyApplicationError::Refused(if room == ReportRoom::Full {
            format!("the queue of {name} is full: {QUEUE_LIMIT} directives wait")
        } else {
            format!("the queue of {name} has no room for reports: {REPORT_SHARE} reports wait")
        })
    }

    fn type_report_now(
        &self,
        report: &ParentReport,
    ) -> Result<DirectiveOutcome, HierarchyApplicationError> {
        match self.type_claimed(report.parent, &report.line, Some(&report.record_text)) {
            Ok(Delivery::Submitted) => Ok(DirectiveOutcome::Submitted),
            Ok(Delivery::Unsubmitted) => Ok(DirectiveOutcome::Unsubmitted),
            Err(message) => Err(HierarchyApplicationError::Conflict(message)),
        }
    }

    fn queue_report(
        &self,
        report: &ParentReport,
    ) -> Result<DirectiveOutcome, HierarchyApplicationError> {
        let waiting = WaitingDirective {
            sender: report.sender,
            target: report.parent,
            text: report.line.clone(),
            accepted_at: self.clock.steady_now(),
            report: Some(report.record_text.clone()),
        };
        match self.queue.push_report(waiting) {
            ReportPush::Replaced { position, replaced } => {
                self.withdrawn(&replaced, REPLACED);
                Ok(DirectiveOutcome::Waiting { position })
            }
            ReportPush::Added { position } => Ok(DirectiveOutcome::Waiting { position }),
            // The queue filled between the room check and the push.
            ReportPush::Refused(room) => Err(self.no_room(report.parent, room)),
            ReportPush::Closed => Err(HierarchyApplicationError::Refused(STOPPING.to_owned())),
        }
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
            report: None,
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
        match self.type_claimed(target, text, None) {
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
    /// the order of its receipts (0011 D2); only a submitted directive is a
    /// received task (0012 PB-3), never a report (0015 PB-3), which carries
    /// its record text in `report`.
    fn type_claimed(
        &self,
        target: AgentId,
        text: &str,
        report: Option<&str>,
    ) -> Result<Delivery, String> {
        let delivery = self.writer.deliver(target, text);
        if report.is_none() && delivery == Ok(Delivery::Submitted) {
            self.receipt.received(target, text);
        }
        self.queue.finish(target, self.clock.steady_now());
        delivery
    }

    /// How many directives and reports wait in the target's queue.
    #[must_use]
    pub fn waiting_count(&self, target: AgentId) -> usize {
        self.queue.waiting_count(target)
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
                let name = self.name_of(target);
                let reason = if self.targets.is_asleep(target) {
                    HierarchyError::Asleep { name }.to_string()
                } else {
                    format!("{name} is no longer running")
                };
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
                match self.type_claimed(target, &head.text, head.report.as_deref()) {
                    Ok(Delivery::Submitted) => {}
                    Ok(Delivery::Unsubmitted) => self.record_failure(head, UNSUBMITTED),
                    Err(message) => {
                        self.withdrawn(head, &format!("the text could not be typed: {message}"));
                    }
                }
            }
        }
    }

    /// Runs the tree rule and the hold check again for a directive that
    /// waited. The hold check counts nothing (0012 A6). A report's tree rule
    /// is that its sender is still a child of the target (0015/FR-005).
    fn recheck(&self, head: &WaitingDirective) -> Recheck {
        let Ok(snapshot) = self.store.snapshot() else {
            return Recheck::Unreadable;
        };
        if head.report.is_some() {
            if report_receiver(&snapshot.rows, head.sender, false)
                != ReportReceiver::Parent(head.target)
            {
                return Recheck::Refused(format!(
                    "the sender is no longer a child of {}",
                    self.name_of(head.target)
                ));
            }
        } else if let Err(error) = check_descendant(&snapshot.rows, head.sender, head.target) {
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
        self.record_failure(directive, &format!("withdrawn before typing: {reason}"));
    }

    /// Records what went wrong with a waiting entry: a directive in the
    /// directive journal, a report in the report journal with its record
    /// text (0015/FR-009).
    fn record_failure(&self, directive: &WaitingDirective, failure: &str) {
        match (&directive.report, &self.report_journal) {
            (Some(record_text), Some(report_journal)) => report_journal.refused(
                directive.sender,
                Some(directive.target),
                record_text,
                failure,
            ),
            _ => self.journal.record(
                directive.sender,
                directive.target,
                &directive.text,
                Some(failure),
            ),
        }
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
