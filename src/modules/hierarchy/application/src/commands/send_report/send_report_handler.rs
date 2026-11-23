// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::{
    AgentAction, ReportKind, ReportReceiver, TreeRow, report_line, report_receiver,
    report_record_text, validate_report_text, visible_text,
};
use kernel::AgentId;

use crate::commands::hold::HoldService;
use crate::commands::send_directive::{
    DirectiveOutcome, ParentReport, SendDirectiveHandler, UNSUBMITTED, not_running_error,
};
use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;
use crate::common::ports::{DirectiveTarget, PersonReports, ReportJournal};

/// One report as the sender sent it: the text is the raw text, before its
/// visible form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub sender: AgentId,
    pub kind: ReportKind,
    pub text: String,
    pub for_person: bool,
}

/// Where an accepted report went (0015 contract, `report.send` result).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportDelivery {
    /// The report is on the sender's Needs you item.
    NeedsYou,
    /// The report went to the parent's terminal, or waits in its queue, as
    /// a directive does (0012/FR-004).
    Parent(DirectiveOutcome),
}

/// Sends one report upward (0015): to the sender's parent, or to the person
/// when the sender has no parent or asks for the person. The text must pass
/// the report rules, the parent must be running, and the holds and the
/// reports limit apply (FR-006). The office turns the text into its visible
/// form once, here, and passes only that form on (FR-003). Every refusal is
/// journaled, so the action record keeps every attempt (FR-009).
pub struct SendReportHandler {
    store: Arc<dyn HierarchyStore>,
    targets: Arc<dyn DirectiveTarget>,
    journal: Arc<dyn ReportJournal>,
    person: Arc<dyn PersonReports>,
    parent_delivery: Arc<SendDirectiveHandler>,
    hold_guard: Option<Arc<HoldService>>,
}

/// The ports a `SendReportHandler` talks to.
pub struct ReportPorts {
    pub store: Arc<dyn HierarchyStore>,
    pub targets: Arc<dyn DirectiveTarget>,
    pub journal: Arc<dyn ReportJournal>,
    pub person: Arc<dyn PersonReports>,
    /// The directive handler, whose queue a report to a parent shares
    /// (0015/FR-004, PB-2).
    pub parent_delivery: Arc<SendDirectiveHandler>,
}

impl SendReportHandler {
    #[must_use]
    pub fn new(ports: ReportPorts) -> Self {
        let ReportPorts {
            store,
            targets,
            journal,
            person,
            parent_delivery,
        } = ports;
        Self {
            store,
            targets,
            journal,
            person,
            parent_delivery,
            hold_guard: None,
        }
    }

    /// Puts the hold and rate-limit gate in front of a report for the
    /// person. The gate journals no refusal; this handler journals each one
    /// (0007). A report to a parent passes the gate of the directive handler
    /// that owns the parent's queue.
    #[must_use]
    pub fn with_hold_guard(mut self, hold_guard: Arc<HoldService>) -> Self {
        self.hold_guard = Some(hold_guard);
        self
    }

    /// # Errors
    /// `Storage` when the tree cannot be read or the parent's state cannot
    /// be read; `Invalid` naming the rule when the text breaks the report
    /// rules or the parent is not running; `Refused` with the reason when
    /// the parent's queue has no room, or a hold or the reports limit stops
    /// it; `Conflict` when the line cannot be typed.
    pub fn handle(&self, report: &Report) -> Result<ReportDelivery, HierarchyApplicationError> {
        let snapshot = self
            .store
            .snapshot()
            .map_err(|error| self.refused(report, None, error))?;
        let parent = match report_receiver(&snapshot.rows, report.sender, report.for_person) {
            ReportReceiver::Parent(parent) => Some(parent),
            ReportReceiver::Person => None,
        };
        if let Err(error) = validate_report_text(&report.text) {
            return Err(self.refused(report, parent, error.into()));
        }
        let text = visible_text(&report.text);
        match parent {
            None => self.send_to_person(report, &text, &snapshot.rows),
            Some(parent) => self.send_to_parent(report, parent, &text, &snapshot.rows),
        }
    }

    fn send_to_person(
        &self,
        report: &Report,
        text: &str,
        rows: &[TreeRow],
    ) -> Result<ReportDelivery, HierarchyApplicationError> {
        self.check_holds(report, rows)?;
        self.person.deliver(report.sender, report.kind, text);
        self.journal.accepted(
            report.sender,
            report.sender,
            &report_record_text(report.kind, text),
        );
        Ok(ReportDelivery::NeedsYou)
    }

    /// Hands the report to the parent's queue as one line with the office's
    /// heading (0015/FR-004, A4), and records it under the parent; a typed
    /// report that got no Enter is recorded as unsubmitted too (0012/FR-003).
    fn send_to_parent(
        &self,
        report: &Report,
        parent: AgentId,
        text: &str,
        rows: &[TreeRow],
    ) -> Result<ReportDelivery, HierarchyApplicationError> {
        self.check_running(report, parent)?;
        let name = self
            .targets
            .name_of(report.sender)
            .unwrap_or_else(|| report.sender.to_string());
        let record_text = report_record_text(report.kind, text);
        let queued = ParentReport {
            sender: report.sender,
            parent,
            line: report_line(&visible_text(&name), report.sender, report.kind, text),
            record_text: record_text.clone(),
        };
        let outcome = self
            .parent_delivery
            .send_report(&queued, rows)
            .map_err(|error| self.refused(report, Some(parent), error))?;
        self.journal.accepted(report.sender, parent, &record_text);
        if outcome == DirectiveOutcome::Unsubmitted {
            self.journal
                .refused(report.sender, Some(parent), &record_text, UNSUBMITTED);
        }
        Ok(ReportDelivery::Parent(outcome))
    }

    /// A report goes only to a running parent (FR-005). A store failure
    /// behind the lookup is `Storage`: the parent may run, but the office
    /// cannot say so.
    fn check_running(
        &self,
        report: &Report,
        parent: AgentId,
    ) -> Result<(), HierarchyApplicationError> {
        let is_running = self.targets.is_running(parent).map_err(|message| {
            self.refused(
                report,
                Some(parent),
                HierarchyApplicationError::Storage(message),
            )
        })?;
        if is_running {
            return Ok(());
        }
        let name = self
            .targets
            .name_of(parent)
            .unwrap_or_else(|| parent.to_string());
        Err(self.refused(
            report,
            Some(parent),
            HierarchyApplicationError::Invalid(not_running_error(&*self.targets, parent, name)),
        ))
    }

    /// The holds and the reports limit for a report for the person, after
    /// every report rule, so a report the rules refuse never counts toward
    /// the limit (0007 A10). It has no target: a hold on the sender or one of
    /// its ancestors refuses it.
    fn check_holds(
        &self,
        report: &Report,
        rows: &[TreeRow],
    ) -> Result<(), HierarchyApplicationError> {
        let Some(hold_guard) = &self.hold_guard else {
            return Ok(());
        };
        hold_guard
            .guard(report.sender, AgentAction::Report, None, rows)
            .map_err(|refusal| {
                self.refused(
                    report,
                    None,
                    HierarchyApplicationError::Refused(refusal.reason),
                )
            })
    }

    /// Journals a refusal with the text in visible form, so no control or
    /// bidirectional format character of a refused report reorders or acts
    /// on the record (0015/FR-003, final review L1).
    fn refused(
        &self,
        report: &Report,
        parent: Option<AgentId>,
        error: HierarchyApplicationError,
    ) -> HierarchyApplicationError {
        self.journal.refused(
            report.sender,
            parent,
            &visible_text(&report.text),
            &error.to_string(),
        );
        error
    }
}

#[cfg(test)]
#[path = "send_report_handler.tests.rs"]
mod tests;
