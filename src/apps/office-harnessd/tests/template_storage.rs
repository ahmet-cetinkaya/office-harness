// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! The template module in the office database: its ledger runs with the other
//! modules', and saving or deleting a template writes no action record
//! (0009 A10).

use std::sync::{Arc, Mutex};

use agent_infrastructure::persistence::{
    LEDGER_TABLE, PRE_MIGRATION_CHECKS, SCRIPTS as AGENT_SCRIPTS,
};
use audit_infrastructure::persistence::{
    LEDGER_TABLE as AUDIT_LEDGER_TABLE, SCRIPTS as AUDIT_SCRIPTS,
};
use kernel::{AgentId, Timestamp};
use office_harnessd::bootstrap::run_migrations;
use rusqlite::Connection;
use template_application::common::errors::TemplateApplicationError;
use template_application::common::interfaces::{AgentSource, TemplateClock};
use template_application::common::models::AgentDefinition;
use template_application::services::TemplateService;
use template_domain::{AdditionsBody, AgentBody, NamePattern, TemplateBody};
use template_infrastructure::persistence::{
    LEDGER_TABLE as TEMPLATE_LEDGER_TABLE, SCRIPTS as TEMPLATE_SCRIPTS, TemplateRepository,
};

struct NoAgents;

impl AgentSource for NoAgents {
    fn definition_of(
        &self,
        agent_id: AgentId,
    ) -> Result<AgentDefinition, TemplateApplicationError> {
        Err(TemplateApplicationError::NotFound(format!(
            "agent {agent_id} does not exist"
        )))
    }
}

struct FixedClock;

impl TemplateClock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(1)
    }
}

fn count(connection: &Mutex<Connection>, table: &str) -> i64 {
    connection
        .lock()
        .expect("lock")
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

#[test]
fn saving_and_deleting_a_template_adds_no_action_record_entry() {
    let mut connection = Connection::open_in_memory().expect("database");
    run_migrations(
        &mut connection,
        LEDGER_TABLE,
        AGENT_SCRIPTS,
        PRE_MIGRATION_CHECKS,
    )
    .expect("agent");
    run_migrations(&mut connection, AUDIT_LEDGER_TABLE, AUDIT_SCRIPTS, &[]).expect("audit");
    run_migrations(
        &mut connection,
        TEMPLATE_LEDGER_TABLE,
        TEMPLATE_SCRIPTS,
        &[],
    )
    .expect("template");
    let connection = Arc::new(Mutex::new(connection));
    let service = TemplateService::new(
        Arc::new(TemplateRepository::new(connection.clone())),
        Arc::new(NoAgents),
        Arc::new(FixedClock),
    );
    let body = TemplateBody::Agent(AgentBody {
        name_pattern: NamePattern::new("x-{n}"),
        command: "claude".to_owned(),
        working_folder: None,
        additions: AdditionsBody::default(),
        role: None,
    });

    let saved = service.save(None, "Reviewer", body).expect("save");
    service.delete(saved.id).expect("delete");

    assert_eq!(count(&connection, "audit_actions"), 0);
    assert_eq!(count(&connection, "template_templates"), 0);
}
