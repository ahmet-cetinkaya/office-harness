// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "tests fail by panicking")]

//! A secret value that an agent prints, runs in a command line, sends in a
//! report or a directive, or names in a field never reaches what the office
//! stores (0026/FR-012): the stored output, the stored process identities, the
//! action record, and Needs you.

mod support;

use rusqlite::Connection;
use serde_json::{Value, json};
use support::office_client::{Client, Office, wait_for};
use support::secret_agents::{
    FIRST_VALUE, HELPER, actions_of, call, caller_of, contains, data_folder_bytes, grant, save,
};

const PLACEHOLDER: &str = "[secret API_KEY]";

/// The CLI's own start: its environment goes to `<records>/<name>.env`.
fn handing_over_the_environment(office: &Office, name: &str) -> String {
    let folder = office.records_path().display();
    format!("env > {folder}/{name}.part\nmv {folder}/{name}.part {folder}/{name}.env\n")
}

/// Reads the value inside the CLI, as the dialog tells an agent to.
fn reading_the_value() -> String {
    format!("VALUE=$({HELPER} agent-call secret.get '{{\"name\":\"API_KEY\"}}' --value-only)\n")
}

/// Saves `API_KEY`, creates an agent with the CLI `body`, grants the secret,
/// and starts it, so that its first read finds the grant.
async fn agent_that_runs(office: &Office, client: &mut Client, name: &str, body: &str) -> String {
    let script = format!(
        "{}{}{body}",
        handing_over_the_environment(office, name),
        reading_the_value()
    );
    office.clis.add(name, &script);
    save(client, "API_KEY", FIRST_VALUE).await;
    let agent_id = client.create_agent(office, name, name, None).await;
    grant(client, "API_KEY", &agent_id).await;
    client.start(&agent_id).await;
    agent_id
}

fn data_folder(office: &Office) -> std::path::PathBuf {
    office
        .serving
        .office
        .config
        .database_path()
        .parent()
        .expect("data folder")
        .to_path_buf()
}

#[tokio::test]
async fn a_value_an_agent_prints_whole_or_in_two_parts_is_stored_as_a_placeholder() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let body = "printf 'before %s after\\n' \"$VALUE\"\n\
                HEAD=$(printf %s \"$VALUE\" | cut -c1-9)\n\
                REST=$(printf %s \"$VALUE\" | cut -c10-)\n\
                printf 'half %s' \"$HEAD\"\nprintf '%s end\\n' \"$REST\"\nsleep 30\n";
    agent_that_runs(&office, &mut client, "printer", body).await;
    let folder = data_folder(&office);

    let stored = wait_for("the printed output in the store", || {
        let bytes = data_folder_bytes(&folder);
        contains(&bytes, &format!("{PLACEHOLDER} end")).then_some(bytes)
    })
    .await;

    assert!(contains(&stored, &format!("before {PLACEHOLDER} after")));
    assert!(contains(&stored, &format!("half {PLACEHOLDER} end")));
    assert!(!contains(&stored, FIRST_VALUE));
    office.serving.stop().await;
}

#[tokio::test]
async fn a_helper_that_carries_the_value_in_its_command_line_is_stored_with_the_placeholder() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let body = "sh -c 'sleep 40; true' \"$VALUE\" &\nsleep 40\n";
    agent_that_runs(&office, &mut client, "runner", body).await;
    let database = office.serving.office.config.database_path();

    let rows = wait_for("a stored helper with the placeholder", || {
        let connection = Connection::open(&database).expect("database");
        let mut statement = connection
            .prepare("SELECT command_display FROM agent_session_processes")
            .expect("prepare");
        let rows: Vec<String> = statement
            .query_map([], |row| row.get(0))
            .expect("query")
            .map(|row| row.expect("row"))
            .collect();
        rows.iter()
            .any(|row| row.contains(PLACEHOLDER))
            .then_some(rows)
    })
    .await;

    assert!(
        rows.iter().all(|row| !row.contains(FIRST_VALUE)),
        "{rows:?}"
    );
    let folder = data_folder(&office);
    let _running = office.serving.stop().await;
    assert!(!contains(&data_folder_bytes(&folder), FIRST_VALUE));
}

#[tokio::test]
async fn a_report_a_refused_directive_and_a_field_name_carry_the_placeholder() {
    let office = Office::open().await;
    let mut client = office.client().await;
    let agent_id = agent_that_runs(&office, &mut client, "talker", "sleep 60\n").await;
    let caller = caller_of(office.records_path(), "talker").await;
    let stranger = "00000000-0000-4000-8000-000000000001";

    let report = call(
        &caller,
        "report.send",
        &json!({"kind": "info", "text": format!("the key is {FIRST_VALUE}"), "for_person": true}),
    )
    .await;
    let directive = call(
        &caller,
        "directive.send",
        &json!({"target_agent_id": stranger, "text": format!("use {FIRST_VALUE} now")}),
    )
    .await;
    let named = call(&caller, "report.send", &json!({FIRST_VALUE: 1})).await;

    assert_eq!(report["ok"], true, "{report}");
    assert_eq!(directive["ok"], false, "{directive}");
    assert_eq!(named["error"]["code"], "invalid", "{named}");
    let items = client.ask("needs_you.list", json!({})).await;
    let item = items["result"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["agent_id"] == agent_id)
        .expect("the item");
    assert_eq!(
        item["report"]["text"],
        format!("the key is {PLACEHOLDER}"),
        "{item}"
    );
    let entries = actions_of(&mut client, &agent_id).await;
    let record: Value = json!(entries);
    assert!(!record.to_string().contains(FIRST_VALUE), "{record}");
    assert!(record.to_string().contains(PLACEHOLDER), "{record}");
    let notifications = client.ask("notification.list", json!({})).await;
    assert!(!notifications.to_string().contains(FIRST_VALUE));
    let folder = data_folder(&office);
    let _running = office.serving.stop().await;
    assert!(!contains(&data_folder_bytes(&folder), FIRST_VALUE));
}

/// Columns that can hold text an agent or its processes produced. Each says
/// where the value is replaced before the column is written (0026/FR-012):
/// `frame` is the agent-call frame, `store` a decorator of the store, `net` the
/// action record net, `derived` a text built from a replaced one.
const AGENT_TEXT_COLUMNS: &[(&str, &str)] = &[
    ("agent_agents.name", "frame"),
    ("agent_agents.name_key", "frame"),
    ("agent_agents.command", "frame"),
    ("agent_agents.task_text", "frame"),
    ("agent_agents.temp_objective", "frame"),
    ("agent_office_additions.additions", "frame"),
    ("agent_finished_temps.name", "derived"),
    ("agent_finished_temps.objective", "derived"),
    ("agent_finished_temps.parent_name", "derived"),
    ("agent_finished_temps.report_text", "derived"),
    ("agent_finished_temps.reason", "derived"),
    ("agent_session_processes.command_display", "store"),
    ("audit_actions.detail", "net"),
    ("audit_actions.result", "net"),
    ("notification_notifications.agent_name", "derived"),
    ("notification_notifications.detail", "derived"),
    ("notification_needs_you.report_text", "derived"),
    ("notification_needs_you.latest_detail", "derived"),
    ("notification_needs_you.report_kind", "derived"),
    ("hierarchy_holds.reason", "derived"),
    ("hierarchy_teams.name", "frame"),
    // The refusal, skip, and failure texts of a run: hold reasons, the wake
    // texts, and the queue answers, built from replaced names and fixed words
    // (0025).
    ("schedule_runs.reason", "derived"),
];

/// Columns that hold ids, fixed words, paths, numbers as text, settings, and
/// what the person typed: no agent-supplied text.
const OTHER_TEXT_COLUMNS: &[&str] = &[
    // What the person typed or set in a schedule, and its fixed words and ids (0025).
    "audit_actions.actor_label",
    "schedule_runs.queue_entry",
    "schedule_runs.result",
    "schedule_runs.schedule_id",
    "schedule_runs.via",
    "schedule_schedules.id",
    "schedule_schedules.name",
    "schedule_schedules.name_key",
    "schedule_schedules.off_reason",
    "schedule_schedules.repeat_date",
    "schedule_schedules.repeat_days",
    "schedule_schedules.repeat_kind",
    "schedule_schedules.repeat_time",
    "schedule_schedules.target_agent_id",
    "schedule_schedules.text",
    "agent_agents.id",
    "agent_agents.machine_id",
    "agent_agents.parent_agent_id",
    "agent_agents.project_id",
    "agent_agents.role",
    "agent_agents.status",
    "agent_agents.task_source",
    "agent_agents.team_id",
    "agent_agents.temp_creator",
    "agent_agents.working_folder",
    "agent_agents.worktree_base",
    "agent_agents.worktree_branch",
    "agent_agents.worktree_folder",
    "agent_agents.worktree_repository",
    "agent_finished_temps.agent_id",
    "agent_finished_temps.creator",
    "agent_finished_temps.id",
    "agent_finished_temps.kept_branch",
    "agent_finished_temps.kept_folder",
    "agent_finished_temps.removal_failure",
    "agent_finished_temps.report_kind",
    "agent_machines.id",
    "agent_machines.name",
    "agent_maintenance.task",
    "agent_office_additions.agent_id",
    "agent_projects.folder",
    "agent_projects.id",
    "agent_projects.name",
    "agent_projects.name_key",
    "agent_session_processes.boot",
    "agent_session_processes.kind",
    "agent_session_processes.session_id",
    "agent_session_processes.state",
    "agent_session_processes.sweep_outcome",
    "agent_sessions.additions_snapshot",
    "agent_sessions.agent_id",
    "agent_sessions.cli_session_id",
    "agent_sessions.continued_from",
    "agent_sessions.exit_result",
    "agent_sessions.id",
    "agent_settings.key",
    "agent_settings.value",
    "audit_actions.action",
    "audit_actions.actor",
    "audit_actions.target_agent_id",
    "hierarchy_holds.scope",
    "hierarchy_holds.set_by",
    "hierarchy_team_members.agent_id",
    "hierarchy_team_members.team_id",
    "hierarchy_teams.id",
    "hierarchy_teams.lead_agent_id",
    "hierarchy_teams.name_key",
    "hierarchy_teams.project_id",
    "notification_needs_you.agent_id",
    "notification_needs_you.latest_kind",
    "notification_needs_you.origin",
    "notification_notifications.agent_id",
    "notification_notifications.event",
    "notification_notifications.id",
    "secret_grants.agent_id",
    "secret_grants.secret_id",
    "secret_secrets.id",
    "secret_secrets.name",
    "secret_store.custody",
    "template_templates.body",
    "template_templates.id",
    "template_templates.kind",
    "template_templates.name",
    "template_templates.name_key",
];

/// Every TEXT column of the office database.
fn text_columns(database: &std::path::Path) -> std::collections::BTreeSet<String> {
    let connection = Connection::open(database).expect("database");
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .expect("tables")
        .query_map([], |row| row.get(0))
        .expect("query")
        .map(|row| row.expect("row"))
        .collect();
    let mut columns = std::collections::BTreeSet::new();
    for table in tables {
        let mut info = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("info");
        let found: Vec<(String, String)> = info
            .query_map([], |row| Ok((row.get(1)?, row.get(2)?)))
            .expect("query")
            .map(|row| row.expect("row"))
            .collect();
        for (column, kind) in found {
            if kind.eq_ignore_ascii_case("TEXT") {
                columns.insert(format!("{table}.{column}"));
            }
        }
    }
    columns
}

/// A column that is added later fails this test until someone decides whether
/// it can hold agent-supplied text, and if it can, where the value is replaced
/// before it is written (0026/FR-012, the enumeration of the storing points).
#[tokio::test]
async fn every_text_column_is_decided_to_hold_agent_text_or_not() {
    let office = Office::open().await;
    let found = text_columns(&office.serving.office.config.database_path());
    let decided: std::collections::BTreeSet<String> = AGENT_TEXT_COLUMNS
        .iter()
        .map(|(column, _)| (*column).to_owned())
        .chain(OTHER_TEXT_COLUMNS.iter().map(|column| (*column).to_owned()))
        .collect();

    let undecided: Vec<&String> = found.difference(&decided).collect();
    let gone: Vec<&String> = decided.difference(&found).collect();

    assert!(
        undecided.is_empty(),
        "decide whether these hold agent-supplied text (0026/FR-012): {undecided:?}"
    );
    assert!(gone.is_empty(), "these columns no longer exist: {gone:?}");
    office.serving.stop().await;
}

#[tokio::test]
async fn output_that_ends_with_the_first_byte_of_a_value_is_not_held_back() {
    // The CLI outlives the wait below, so only the release after a pause, not
    // the end of the output, can store the last byte.
    let office = Office::open().await;
    let mut client = office.client().await;
    let body = "FIRST=$(printf %s \"$VALUE\" | cut -c1-1)\n\
                printf 'last byte %s' \"$FIRST\"\nsleep 120\n";
    agent_that_runs(&office, &mut client, "pauser", body).await;
    let folder = data_folder(&office);

    let stored = wait_for("the last byte in the stored output", || {
        let bytes = data_folder_bytes(&folder);
        contains(&bytes, &format!("last byte {}", &FIRST_VALUE[..1])).then_some(bytes)
    })
    .await;

    assert!(!contains(&stored, FIRST_VALUE));
    office.serving.stop().await;
}
