// SPDX-License-Identifier: AGPL-3.0-or-later

use hierarchy_application::common::ports::PersonReports;
use hierarchy_domain::ReportKind;
use kernel::AgentId;

use super::NotificationBridge;

/// Hands a report for the person to the notification bridge, which
/// notifies the owner and puts the report on the sender's Needs you item
/// (0015/FR-007, FR-008).
pub struct PersonReportsBridge {
    notifications: NotificationBridge,
}

impl PersonReportsBridge {
    #[must_use]
    pub fn new(notifications: NotificationBridge) -> Self {
        Self { notifications }
    }
}

impl PersonReports for PersonReportsBridge {
    fn deliver(&self, sender: AgentId, kind: ReportKind, text: &str) {
        self.notifications
            .record_report(sender, kind.as_str(), text.to_owned());
    }
}
