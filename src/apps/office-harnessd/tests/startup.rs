// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::expect_used, reason = "tests fail by panicking")]

mod support;

use agent_domain::enums::AgentStatus;
use agent_domain::value_objects::{SettingKey, Settings};
use office_harnessd::bootstrap::{self, Services};
use office_harnessd::config::Config;
use support::{agent_named, config_in};

fn start_server_state(config: &Config) -> Services {
    let log = bootstrap::open_log(config).expect("log");
    bootstrap::start(config, &log).expect("start")
}

#[test]
fn definitions_and_settings_survive_a_restart() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let first = start_server_state(&config);
    let agent = agent_named(first.machine.id, "Ana");
    first.agents.insert(&agent).expect("agent");
    let mut settings = Settings::default();
    settings
        .set_from_text(SettingKey::StopGraceMs, "2500")
        .expect("setting");
    first.settings.save(&settings).expect("settings");
    drop(first);

    let second = start_server_state(&config);

    assert_eq!(second.agents.list().expect("agents"), [agent]);
    assert_eq!(second.settings.load().expect("settings"), settings);
}

#[test]
fn running_agents_become_interrupted_at_start_and_none_starts_by_itself() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let first = start_server_state(&config);
    let agent = agent_named(first.machine.id, "Ana");
    first.agents.insert(&agent).expect("agent");
    first
        .agents
        .update_status(agent.id, AgentStatus::Working)
        .expect("status");
    drop(first);

    let second = start_server_state(&config);

    let status = second
        .agents
        .find(agent.id)
        .expect("find")
        .expect("exists")
        .status;
    assert_eq!(status, AgentStatus::Interrupted);
}

#[cfg(unix)]
#[test]
fn the_data_folder_and_its_files_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    drop(start_server_state(&config));

    let mode = |path: &std::path::Path| {
        std::fs::metadata(path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode(&config.data_folder), 0o700);
    assert_eq!(mode(&config.database_path()), 0o600);
    assert_eq!(mode(&config.log_path()), 0o600);
}

/// Writes the place columns by hand, as an office saved before 0002 A5 was
/// fixed holds them: a role and no parent.
fn place_by_hand(config: &Config, agent: &agent_domain::entities::Agent, role: &str) {
    let connection = rusqlite::Connection::open(config.database_path()).expect("database");
    connection
        .execute(
            "UPDATE agent_agents SET role = ?1, parent_agent_id = NULL WHERE id = ?2",
            rusqlite::params![role, agent.id.as_uuid_text()],
        )
        .expect("place");
}

fn parent_of(config: &Config, agent: &agent_domain::entities::Agent) -> Option<String> {
    let connection = rusqlite::Connection::open(config.database_path()).expect("database");
    connection
        .query_row(
            "SELECT parent_agent_id FROM agent_agents WHERE id = ?1",
            rusqlite::params![agent.id.as_uuid_text()],
            |row| row.get(0),
        )
        .expect("agent row")
}

/// 0002 A5 for an office saved before the fix: at the next start the leads
/// that have no parent move under the only CEO, and a second start changes
/// nothing more.
#[test]
fn a_ceo_and_parentless_leads_from_before_the_fix_are_repaired_at_start() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let first = start_server_state(&config);
    let ceo = agent_named(first.machine.id, "Cy");
    let lead = agent_named(first.machine.id, "Lea");
    first.agents.insert(&ceo).expect("CEO");
    first.agents.insert(&lead).expect("lead");
    drop(first);
    place_by_hand(&config, &ceo, "ceo");
    place_by_hand(&config, &lead, "lead");

    drop(start_server_state(&config));
    assert_eq!(parent_of(&config, &lead), Some(ceo.id.as_uuid_text()));
    drop(start_server_state(&config));

    assert_eq!(parent_of(&config, &lead), Some(ceo.id.as_uuid_text()));
    assert_eq!(parent_of(&config, &ceo), None);
}

/// The repair needs exactly one CEO: with two (a hand-edited office), no
/// lead is moved, because the right parent is unknown.
#[test]
fn the_start_repair_moves_no_lead_when_two_ceos_exist() {
    let folder = tempfile::tempdir().expect("folder");
    let config = config_in(&folder);
    let first = start_server_state(&config);
    let agents: Vec<_> = ["Cy", "Cal", "Lea"]
        .into_iter()
        .map(|name| agent_named(first.machine.id, name))
        .collect();
    for agent in &agents {
        first.agents.insert(agent).expect("agent");
    }
    drop(first);
    place_by_hand(&config, &agents[0], "ceo");
    place_by_hand(&config, &agents[1], "ceo");
    place_by_hand(&config, &agents[2], "lead");

    drop(start_server_state(&config));

    assert_eq!(parent_of(&config, &agents[2]), None);
}
