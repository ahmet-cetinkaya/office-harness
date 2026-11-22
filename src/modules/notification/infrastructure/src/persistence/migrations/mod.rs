// SPDX-License-Identifier: AGPL-3.0-or-later

/// The ledger table that records how many scripts of this module ran.
pub const LEDGER_TABLE: &str = "notification_schema_version";

/// The forward-only migration scripts in order. Script `n` brings the module's
/// tables to version `n`. A shipped script never changes; a new one is added
/// at the end.
pub const SCRIPTS: &[&str] = &[
    include_str!("0001_notifications.sql"),
    include_str!("0002_detail.sql"),
    include_str!("0003_needs_you.sql"),
    include_str!("0004_needs_you_report.sql"),
];
