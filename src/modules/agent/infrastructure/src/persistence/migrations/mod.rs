// SPDX-License-Identifier: AGPL-3.0-or-later

mod name_key_checks;

use rusqlite::Connection;

/// A data check that runs before any pending script, while the ledger is
/// below the version it guards. The check reads the database and returns the
/// user-facing problem text, or `Ok(())` when the data is ready for the
/// scripts of its version and the ones after it.
pub type PreMigrationCheck = (i64, fn(&Connection) -> Result<(), String>);

pub use name_key_checks::check_name_key_ascii_groups;

/// The ledger table that records how many scripts of this module ran.
pub const LEDGER_TABLE: &str = "agent_schema_version";

/// The forward-only migration scripts in order. Script `n` brings the module's
/// tables to version `n`. A shipped script never changes; a new one is added
/// at the end.
pub const SCRIPTS: &[&str] = &[
    include_str!("0001_machines_projects_agents.sql"),
    include_str!("0002_settings.sql"),
    include_str!("0003_additions_and_sessions.sql"),
    include_str!("0004_name_key_nocase.sql"),
    // Plain columns with no foreign key, so it does not depend on the
    // hierarchy module's tables existing first.
    include_str!("0005_hierarchy_columns.sql"),
    include_str!("0006_task_text.sql"),
    include_str!("0007_agent_worktree.sql"),
];

/// The data checks the server runs before it steps a database forward. Each
/// entry guards one version and runs only while the ledger is below it, so a
/// hand-edited database fails with a clear message and no data changes.
pub const PRE_MIGRATION_CHECKS: &[PreMigrationCheck] = &[(
    name_key_checks::NAME_KEY_INDEX_VERSION,
    check_name_key_ascii_groups,
)];
