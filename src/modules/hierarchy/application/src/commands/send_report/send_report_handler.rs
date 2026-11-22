// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use hierarchy_domain::{
    AgentAction, HierarchyError, ReportKind, ReportReceiver, TreeRow, report_receiver,
    report_record_text, validate_report_text, visible_text,
};
use kernel::AgentId;

use crate::commands::hold::HoldService;
use crate::common::errors::HierarchyApplicationError;
use crate::common::interfaces::HierarchyStore;
use crate::common::ports::{DirectiveTarget, PersonReports, ReportJournal};

/// Why a report to a parent is refused until its delivery through the
/// directive queue of 0012 is built (0015 PLAN T5).
const PARENT_DELIVERY_NOT_BUILT: &str = "reports to a parent are not delivered yet";

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
    hold_guard: Option<Arc<HoldService>>,
}

impl SendReportHandler {
    #[must_use]
    pub fn new(
        store: Arc<dyn HierarchyStore>,
        targets: Arc<dyn DirectiveTarget>,
        journal: Arc<dyn ReportJournal>,
        person: Arc<dyn PersonReports>,
    ) -> Self {
        Self {
            store,
            targets,
            journal,
            person,
            hold_guard: None,
        }
    }

    /// Puts the hold and rate-limit gate in front of the delivery. The gate
    /// journals no refusal; this handler journals each one (0007).
    #[must_use]
    pub fn with_hold_guard(mut self, hold_guard: Arc<HoldService>) -> Self {
        self.hold_guard = Some(hold_guard);
        self
    }

    /// # Errors
    /// `Storage` when the tree cannot be read or the parent's state cannot
    /// be read; `Invalid` naming the rule when the text breaks the report
    /// rules or the parent is not running; `Refused` with the reason when a
    /// hold or the reports limit stops it.
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
        if let Some(parent) = parent {
            self.check_running(report, parent)?;
        }
        self.check_holds(report, parent, &snapshot.rows)?;
        let text = visible_text(&report.text);
        match parent {
            None => {
                self.person.deliver(report.sender, report.kind, &text);
                self.journal.accepted(
                    report.sender,
                    report.sender,
                    &report_record_text(report.kind, &text),
                );
                Ok(ReportDelivery::NeedsYou)
            }
            Some(parent) => Err(self.refused(
                report,
                Some(parent),
                HierarchyApplicationError::Refused(PARENT_DELIVERY_NOT_BUILT.to_owned()),
            )),
        }
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
            HierarchyApplicationError::Invalid(HierarchyError::NotRunning { name }),
        ))
    }

    /// The holds and the reports limit, after every report rule, so a report
    /// the rules refuse never counts toward the limit (0007 A10). The target
    /// is the parent, so a hold on it refuses too; a report for the person
    /// has no target, and a hold on the sender or an ancestor refuses it.
    fn check_holds(
        &self,
        report: &Report,
        parent: Option<AgentId>,
        rows: &[TreeRow],
    ) -> Result<(), HierarchyApplicationError> {
        let Some(hold_guard) = &self.hold_guard else {
            return Ok(());
        };
        hold_guard
            .guard(report.sender, AgentAction::Report, parent, rows)
            .map_err(|refusal| {
                self.refused(
                    report,
                    parent,
                    HierarchyApplicationError::Refused(refusal.reason),
                )
            })
    }

    fn refused(
        &self,
        report: &Report,
        parent: Option<AgentId>,
        error: HierarchyApplicationError,
    ) -> HierarchyApplicationError {
        self.journal
            .refused(report.sender, parent, &report.text, &error.to_string());
        error
    }
}

#[cfg(test)]
#[path = "send_report_handler.tests.rs"]
mod tests;
