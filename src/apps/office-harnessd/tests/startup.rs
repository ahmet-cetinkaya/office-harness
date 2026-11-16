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
