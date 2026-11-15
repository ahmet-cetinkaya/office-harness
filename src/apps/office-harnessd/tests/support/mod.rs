// SPDX-License-Identifier: AGPL-3.0-or-later

use agent_domain::entities::Agent;
use agent_domain::value_objects::{AgentCommand, DisplayName, MachineId};
use kernel::{AgentId, Timestamp};
use office_harnessd::config::Config;
use tempfile::TempDir;

pub fn config_in(folder: &TempDir) -> Config {
    Config {
        data_folder: folder.path().join("office"),
    }
}

pub fn agent_named(machine_id: MachineId, name: &str) -> Agent {
    Agent::new(
        AgentId::generate(),
        DisplayName::parse("name", name).expect("name"),
        AgentCommand::parse("claude").expect("command"),
        Some(std::env::temp_dir()),
        None,
        machine_id,
        Timestamp::from_unix_millis(1_700_000_000_000),
    )
    .expect("agent")
}
